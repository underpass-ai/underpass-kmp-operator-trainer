# Operator v8.0 state explainer — 2026-05-23

Este documento explica, en lenguaje directo, qué pasó desde que arrancamos el entrenamiento del 0.5B, qué artifacts existen, qué funciona, qué no funciona y qué bloquea el cierre de v8.0.

No se han hecho cambios de código, cluster, training ni paid calls para escribir este reporte. Solo se han leído archivos, Kubernetes y se ha reproducido localmente el error del evaluador sobre artifacts ya existentes.

## 1. Timeline desde "vamos a entrenar"

### 1.1 Preflight local y cluster

Qué pasó: se comprobó que existían los scripts de SFT, que había dataset de v7.3.1, que el modelo base era `Qwen/Qwen2.5-0.5B-Instruct`, y que el nodo tenía 4 RTX 3090.

Qué decisión hubo detrás: el plan v8.0 era entrenar primero el 0.5B, sin saltar a modelos más grandes hasta tener evidencia empírica.

Resultado: el nodo era compatible, pero las 4 GPUs estaban ocupadas por un deployment vLLM grande. Eso motivó liberar GPUs.

Desviación: liberar GPUs tocó infraestructura de cluster durante una sesión que originalmente era de training/eval.

### 1.2 Side effects de cluster: DNS/Ingress 0.5B y Gemma a 0

Qué pasó: se preparó `0.5b.llm.underpassai.com` en DNS/TLS/Ingress, y se escaló `underpass-llm-gemma-4-31b-structured` a 0 réplicas.

Qué significa: `0.5b.llm.underpassai.com` es una puerta HTTPS/mTLS preparada en el Ingress, pero no hay un Pod vLLM 0.5B detrás. Ahora mismo apunta al Service `underpass-llm-gemma-4-31b-structured`, que no tiene endpoints porque su Deployment está a 0.

Por qué se hizo: la intención fue preparar el host 0.5B y liberar las GPUs para el entrenamiento. Fue intencional, pero quedó como side effect operativo que no pertenece estrictamente al resultado del modelo.

Resultado: las GPUs quedaron libres tras el training, pero el servicio structured de Gemma quedó parado.

### 1.3 Dataset prep

Qué pasó: se preparó el dataset SFT a partir del corpus v7.3.1:

- Input: `realistic-v7-full-v5-1-pr41-20260523T101100Z/trajectories.jsonl`
- Train: 1,371 rows
- Eval: 242 rows

Qué decisión hubo detrás: usar el corpus que cerró v7.3.1 con gate verde, no regenerar ni cambiar escenarios.

Resultado: dataset usable en `/tmp/operator-sft-v8.0/`.

Sobre "qué falló el primer intento": no encuentro evidencia persistida de un fallo real de dataset prep. Lo que sí ocurrió fue que los comandos del prompt no coincidían exactamente con los flags reales de los scripts, así que se usaron los flags soportados por la rama actual. Si hubo un intento fallido previo, no quedó artifact claro que pueda citar.

### 1.4 Primer intento de Job 1-GPU

Qué pasó: se lanzó inicialmente el manifest repo `k8s/qwen05-lora-train.yaml`.

Qué tenía ese manifest: un Job 1-GPU llamado `operator-qwen05-lora-train`, batch size 2, grad accumulation 8, fp16. Está definido en `k8s/qwen05-lora-train.yaml:22-100`.

Qué decisión hubo detrás: era el manifest existente para entrenar Qwen 0.5B.

Resultado: se borró ese intento y se sustituyó por un manifest scratch 4-GPU. No queda Job 1-GPU activo de esta ejecución.

Desviación: el plan original no pedía un intento 1-GPU seguido de cambio a 4-GPU.

### 1.5 Switch a 4-GPU

Qué pasó: se creó `/tmp/operator-qwen05-lora-train-4gpu.yaml` y se lanzó un Job 4-GPU:

- Job: `operator-qwen05-lora-train-4gpu`
- Namespace: `underpass-runtime`
- GPUs: 4
- `torchrun --standalone --nproc_per_node=4`
- batch size 4 por GPU
- grad accumulation 1
- effective batch 16

Qué decisión hubo detrás: usar las 4 RTX 3090 disponibles para terminar rápido manteniendo el mismo batch efectivo del preset.

Resultado: el Job completó correctamente.

### 1.6 Training run

Qué pasó: el entrenamiento terminó en unos 12 minutos.

Resultado principal:

- Final adapter: `/tmp/operator-qwen05-lora-v8.0/adapter_model.safetensors`
- SHA-256: `4a5ed6fa2057cb2f20db3289fc51ae114ad32167c4f13db1cfb68a3c8855f7b1`
- Steps: 258
- Epochs: 3

Eval por epoch:

| Epoch | Step | eval_loss | eval_mean_token_accuracy |
| --- | ---: | ---: | ---: |
| 1 | 86 | 0.0391338058 | 0.9870025739 |
| 2 | 172 | 0.0242082980 | 0.9899247177 |
| 3 | 258 | 0.0235070214 | 0.9900132008 |

Interpretación humana: el loss bajo indica que el modelo aprendió a imitar el formato del dataset, pero eso no prueba aún que elija bien acciones. Para eso faltaba Phase 4: predicción + evaluación.

Desviaciones importantes:

- El frontier ceiling se saltó antes del training.
- No se aplicó el patch de TensorBoard/step-level eval.
- No se lanzó observer agent.
- Se monitorizó manualmente por logs de Kubernetes.

### 1.7 Phase 2 frontier ceiling retroactive

Qué pasó: después del training, se ejecutó el frontier ceiling con `gpt-4o-mini` sobre el mismo eval split de 242 filas.

Qué significa "frontier ceiling": preguntar a un modelo fuerte, aquí `gpt-4o-mini`, qué acción elegiría en el mismo eval set. Sirve como techo de comparación: si el modelo grande tampoco puede acertar, el dataset/eval puede ser ambiguo.

Resultado API:

- 242/242 llamadas completadas
- 0 fallos de API
- Output: `../rehydration-kernel-artifacts/operator/frontier-ceiling-v8.0-20260523T140341Z/`

Desviación: se hizo retroactivo. Es aceptable para comparar porque usa exactamente el mismo eval split, pero rompe el orden ideal del plan.

### 1.8 Intento de `policy_eval`

Qué pasó: se intentó puntuar las predicciones frontier con `operator-policy-eval`.

Resultado: el evaluador compiló y arrancó, pero abortó al leer la línea 15 de `predictions.jsonl`.

Por qué: esa línea contiene un `kernel_ingest` con `observed_at: "..."`, que no es una fecha RFC3339 válida. El reader convierte cada predicción a tipos de dominio estrictos antes de puntuar. Al encontrar una acción inválida, devuelve error y el CLI se para.

Interpretación humana: no es que la llamada a OpenAI fallara. OpenAI respondió. Lo que falló fue que la respuesta de OpenAI no cumple el contrato estricto.

### 1.9 Intento de Phase 4 predict

Qué pasó: antes de ejecutar predicción del modelo entrenado, se revisó `scripts/operator/predict_operator_sft.py --help`.

Resultado: el script no soporta constrained decoding.

Qué significa "constrained decoding": forzar al modelo, durante la generación, a emitir solo JSON que cumpla el schema. No es lo mismo que "paro cuando veo un JSON".

El script solo tiene `--stop-after-json`, que corta tras el primer objeto JSON completo. Eso ayuda a no generar texto extra, pero no impide que el JSON sea inválido.

Decisión: no se ejecutó Phase 4 porque hacerlo sin constrained decoding violaría la regla explícita del plan.

## 2. Inventario actual de artifacts

| Tipo | Path | SHA o size | Estado | Sirve para qué |
| --- | --- | --- | --- | --- |
| SFT train | `/tmp/operator-sft-v8.0/openai_train.jsonl` | 1,371 rows, SHA `ca6751f48cd3f9c01ae6b56558b5f99df90dab57dfc1e381bc97a4f3f67eab15` | Completo y usable | Entrenar LoRA |
| SFT eval | `/tmp/operator-sft-v8.0/openai_eval.jsonl` | 242 rows, SHA `626eec90c827296c405d75b2395316e3dfe3370ea6fd3d6934906427ec403212` | Completo y usable | Eval frontier/trained |
| SFT summary | `/tmp/operator-sft-v8.0/summary.json` | 27,160 bytes | Completo | Auditoría de split y cobertura |
| Adapter final | `/tmp/operator-qwen05-lora-v8.0/adapter_model.safetensors` | SHA `4a5ed6fa2057cb2f20db3289fc51ae114ad32167c4f13db1cfb68a3c8855f7b1` | Completo y usable | Checkpoint v8.0 entrenado |
| Checkpoint epoch 1 | `/tmp/operator-qwen05-lora-v8.0/checkpoint-86/` | Directory | Completo | Comparar epoch 1 si se decide |
| Checkpoint epoch 2 | `/tmp/operator-qwen05-lora-v8.0/checkpoint-172/` | Directory | Completo | Comparar epoch 2 vs epoch 3 |
| Checkpoint epoch 3 | `/tmp/operator-qwen05-lora-v8.0/checkpoint-258/` | Directory | Completo | Checkpoint final |
| Frontier predictions | `../rehydration-kernel-artifacts/operator/frontier-ceiling-v8.0-20260523T140341Z/predictions.jsonl` | 83,131 bytes | Parcial para scoring: API completa, contrato inválido en algunas rows | Ceiling no-oficial / debugging |
| Frontier failures | `../rehydration-kernel-artifacts/operator/frontier-ceiling-v8.0-20260523T140341Z/failures.jsonl` | 0 bytes | Completo | Confirma 0 fallos API |
| Frontier summary | `../rehydration-kernel-artifacts/operator/frontier-ceiling-v8.0-20260523T140341Z/summary.json` | 315 bytes | Completo | Metadata del run frontier |
| Policy eval report | N/A | N/A | No existe completo | El CLI aborta antes de report |
| 4-GPU scratch manifest | `/tmp/operator-qwen05-lora-train-4gpu.yaml` | 2,962 bytes | Scratch, no repo | Reproducir Job 4-GPU |
| Preflight scratch manifest | `/tmp/operator-sft-preflight-v8.yaml` | 1,845 bytes | Scratch | Preflight SFT |
| Route53 0.5B scratch | `/tmp/route53-0-5b-llm-upsert.json` | 346 bytes | Scratch | Evidencia DNS 0.5B |
| Route53 2.5B scratch | `/tmp/route53-2-5b-llm-upsert.json` | 356 bytes | Scratch | Evidencia DNS 2.5B |
| K8s training Job | `underpass-runtime/operator-qwen05-lora-train-4gpu` | Complete 1/1 | Completo | Job que produjo el adapter |
| K8s training Pod | `operator-qwen05-lora-train-4gpu-tc88n` | Completed | Completo | Pod histórico del training |
| 0.5B serving Pod | N/A | N/A | No existe | No hay modelo 0.5B servido ahora |
| Gemma structured deployment | `underpass-llm-gemma-4-31b-structured` | replicas 0 | Parado | Backend al que apunta el Ingress, sin endpoints |

## 3. EL EVALUADOR — sección crítica

### 3.1 ¿Qué es el evaluador?

El evaluador es el binario Rust `operator-policy-eval`, en crate `operator-evaluation-cli`. Lee dos JSONL:

1. `predictions.jsonl`: lo que predijo un modelo.
2. `trajectories.jsonl`: la verdad esperada.

Cuando funciona, junta ambas por `step_id`, compara acción predicha vs acción esperada, valida si la acción predicha cumple el contrato estricto y muestra tasas como exact match, tool match y contract valid.

### 3.2 ¿Funcionó alguna vez en algún v anterior?

Sí, con inputs válidos controlados.

Evidencias:

- `crates/operator-evaluation-cli/tests/cli_smoke.rs:52-82` prueba que una predicción exacta pasa el threshold.
- `crates/operator-evaluation-cli/tests/cli_smoke.rs:84-115` prueba que una predicción válida pero con target incorrecto falla el threshold.
- `scripts/operator/round_trip_smoke.sh:110-113` lo usa como oracle smoke con `--min-pass-rate 1.0`.
- `crates/operator-synthetic-cli/tests/python_pipeline_full_modes.rs:224-250` también ejecuta `operator-policy-eval` en el pipeline test.
- `docs/training/model-history.md:34-36` documenta el baseline v4 de 24.1% exact-action usando un artifact de policy eval.

Lo que no estaba cubierto suficientemente: una predicción frontier inválida dentro de un archivo grande. Hoy el reader aborta todo el report en la primera predicción inválida.

### 3.3 Comportamiento ACTUAL ejecutándolo hoy

Comando reproducido:

```bash
cargo run --release -p operator-evaluation-cli --bin operator-policy-eval -- \
  --predictions ../rehydration-kernel-artifacts/operator/frontier-ceiling-v8.0-20260523T140341Z/predictions.jsonl \
  --ground-truth ../rehydration-kernel-artifacts/operator/realistic-v7-full-v5-1-pr41-20260523T101100Z/trajectories.jsonl
```

Output literal:

```text
    Finished `release` profile [optimized] target(s) in 0.08s
     Running `target/release/operator-policy-eval --predictions ../rehydration-kernel-artifacts/operator/frontier-ceiling-v8.0-20260523T140341Z/predictions.jsonl --ground-truth ../rehydration-kernel-artifacts/operator/realistic-v7-full-v5-1-pr41-20260523T101100Z/trajectories.jsonl`
policy-eval failed: read predictions: predictions reader 'jsonl_predictions_reader' shape violation at line 15: action: tool 'kernel_ingest' arguments shape is invalid: ingest.provenance.observed_at must be RFC3339: premature end of input
```

### 3.4 ¿El evaluador "funciona" o "no funciona"?

| Pregunta | Respuesta |
| --- | --- |
| ¿Compila y ejecuta? | Sí. El binario arranca. |
| ¿Procesa predicciones válidas correctamente? | Sí, hay tests CLI y use-case con inputs válidos. |
| ¿Maneja predicciones inválidas con gracia? | No. Aborta en la primera shape violation. |
| ¿Es esto bug o decisión de diseño? | Mi lectura: era una decisión razonable para fixtures/oracle, pero es insuficiente para frontier/trained runs reales. En evaluación real, una predicción inválida debería contar como fila inválida, no tumbar el reporte completo. |
| ¿Hay tests que muestren el comportamiento esperado? | Hay tests para inputs válidos, JSON inválido, missing step_id y errores de overlap. No vi un test que exija "continúa tras una acción inválida y cuenta contract_valid=false". |

### 3.5 Dónde está la decisión "abort on shape violation"

El abort nace en el reader:

- `crates/operator-evaluation-infra/src/adapters/jsonl_predictions_reader.rs:57-62`: parsea cada línea JSON.
- `crates/operator-evaluation-infra/src/adapters/jsonl_predictions_reader.rs:70-76`: convierte `OperatorActionDto` a dominio; si falla, retorna `PredictionsReadError::ShapeViolation`.
- `crates/operator-evaluation-cli/src/bin/operator_policy_eval.rs:64-67`: el CLI llama `.read()` y propaga cualquier error como `policy-eval failed`.

El use case sí podría evaluar filas ya parseadas:

- `crates/operator-evaluation-application/src/use_cases/evaluate_operator_policy_use_case.rs:24-50`: recorre pares válidos y produce `EvaluationReport`.

El problema está antes: las filas inválidas no llegan al use case.

### 3.6 Si no abortara con esos ingest inválidos, ¿qué número saldría?

Probablemente sería de la misma escala que el 37% raw exact, quizá algo mayor, pero no esperaría que saltara a 75-92% solo por arreglar el abort.

Razón:

- El scan bruto encontró `kind/tool match = 242/242 = 100%`: el frontier elige la herramienta correcta en todas las filas.
- Pero `raw_exact = 90/242 = 37.19%`: falla mucho copiando argumentos exactos.
- El evaluator oficial puede normalizar algunas diferencias por tipos de dominio, pero no va a convertir placeholders como `observed_at: "..."` en timestamps reales, ni va a reconstruir payloads completos.

Lectura honesta: el ceiling oficial será más informativo cuando el evaluator cuente inválidas como filas inválidas, pero el patrón ya es claro: el frontier sabe elegir tool, no está copiando payloads exactos en muchas acciones estructuradas.

## 4. Frontier ceiling no-oficial — qué hacer con ese 37%

El 37% no es el número oficial. Es una comparación bruta JSON-vs-JSON hecha para entender qué estaba pasando después de que `operator-policy-eval` abortara.

Qué NO incluye:

- No usa el evaluator oficial.
- No aplica normalización semántica.
- No usa accepted actions.
- No separa contract-valid vs exact-mismatch.
- No produce las métricas finales de policy eval.

Qué SÍ nos dice:

- El frontier seleccionó la tool correcta en 100% de filas.
- La exactitud de argumentos completos es baja, especialmente en acciones con payload grande (`kernel_ingest`, `kernel_write_memory`, `kernel_forward`, `kernel_trace`, `kernel_near`, etc.).
- Hay al menos 12 `kernel_ingest` inválidos por `observed_at: "..."`.

¿Es fiable para Phase 5 ahora mismo?

No como cierre oficial. Sí como diagnóstico.

Puede justificar esta decisión técnica: antes de comparar el LoRA entrenado, necesitamos que el evaluator soporte predicciones inválidas como resultado evaluable. Si no, cualquier modelo que emita una sola acción inválida tumba todo el report.

No lo usaría para decidir "v8.0 cierra" ni "0.5B falla". Sí lo usaría para priorizar el arreglo del evaluator.

## 5. Cluster side effects clarificados

### 5.1 `0.5b.llm.underpassai.com`

Comando:

```bash
kubectl -n underpass-runtime get pods --selector='app.kubernetes.io/name in (vllm-qwen05,underpass-llm-qwen05)' -o wide
```

Output:

```text
No resources found in underpass-runtime namespace.
```

Comando:

```bash
kubectl -n underpass-runtime get svc | grep -i 'qwen05\|0-5b'
```

Output:

```text
```

No hay ningún Service qwen05/0-5b.

Comando:

```bash
kubectl -n underpass-runtime get ingress underpass-runtime-vllm -o yaml | grep -B 2 -A 8 '0.5b'
```

Output:

```text
        path: /
        pathType: Prefix
  - host: 0.5b.llm.underpassai.com
    http:
      paths:
      - backend:
          service:
            name: underpass-llm-gemma-4-31b-structured
            port:
              number: 8000
        path: /
--
    - llm.underpassai.com
    - vllm.underpassai.com
    - 0.5b.llm.underpassai.com
    secretName: vllm-tls
status:
  loadBalancer:
    ingress:
    - ip: 192.168.1.241
```

Conclusión:

- No hay Pod vLLM 0.5B corriendo.
- No hay Service qwen05.
- `0.5b.llm.underpassai.com` es DNS/TLS/Ingress preparado, pero apunta al Service Gemma structured.
- Ese Service no tiene backend porque su Deployment está a 0.

TLS/mTLS:

- Ingress annotations incluyen `cert-manager.io/cluster-issuer: letsencrypt-prod-r53`.
- Ingress annotations incluyen `nginx.ingress.kubernetes.io/auth-tls-verify-client: on`.
- `Certificate/vllm-tls` incluye `0.5b.llm.underpassai.com` y está `Ready=True`.

Por qué se hizo durante training: por la conversación previa, se preparó el host 0.5B pensando en servir el LoRA después. Fue prematuro porque el Pod vLLM 0.5B no llegó a desplegarse.

### 5.2 `underpass-llm-gemma-4-31b-structured` escalado a 0

Comando:

```bash
kubectl -n underpass-runtime get deploy underpass-llm-gemma-4-31b-structured -o yaml | head -30
```

Output:

```text
apiVersion: apps/v1
kind: Deployment
metadata:
  annotations:
    deployment.kubernetes.io/revision: "1"
    meta.helm.sh/release-name: underpass-llm-gemma-4-31b
    meta.helm.sh/release-namespace: underpass-runtime
  creationTimestamp: "2026-04-22T08:51:50Z"
  generation: 6
  labels:
    app.kubernetes.io/component: structured
    app.kubernetes.io/instance: underpass-llm-gemma-4-31b
    app.kubernetes.io/managed-by: Helm
    app.kubernetes.io/name: underpass-llm
    helm.sh/chart: underpass-llm-0.2.0
  name: underpass-llm-gemma-4-31b-structured
  namespace: underpass-runtime
  resourceVersion: "25730522"
  uid: 668cca98-6e12-4238-93ef-8f186ec56611
spec:
  progressDeadlineSeconds: 600
  replicas: 0
  revisionHistoryLimit: 10
  selector:
    matchLabels:
      app.kubernetes.io/component: structured
      app.kubernetes.io/instance: underpass-llm-gemma-4-31b
  strategy:
    type: Recreate
  template:
```

Services relacionados:

```text
underpass-llm-gemma-4-31b-orchestrator   ClusterIP   10.107.177.216   <none>        8080/TCP                     31d
underpass-llm-gemma-4-31b-structured     ClusterIP   10.109.132.15    <none>        8000/TCP                     31d
```

Deployments actuales:

```text
NAME                                     READY   UP-TO-DATE   AVAILABLE   AGE
underpass-llm-gemma-4-31b-orchestrator   1/1     1            1           31d
underpass-llm-gemma-4-31b-structured     0/0     0            0           31d
```

Qué workload servía antes: por labels/Helm release, era el componente `structured` de `underpass-llm-gemma-4-31b`, servido en puerto 8000. No inspeccioné logs históricos ni valores Helm, así que no afirmo más que eso.

Por qué se escaló a 0: para liberar GPUs. El estado inicial mostrado por el usuario tenía 4 workers vLLM ocupando ~22 GB por GPU.

¿Hay servicios dependientes? Sí: el Service `underpass-llm-gemma-4-31b-structured` existe y el Ingress apunta a él. Como el Deployment está a 0, el Service no tiene endpoints.

¿Quedó así intencionalmente? Sí, quedó así como consecuencia intencional de liberar GPUs. Operativamente hay que decidir si restaurarlo o reemplazarlo por un deployment 0.5B real.

## 6. Bloqueadores actuales — en palabras llanas

| Bloqueador | Qué impide | Qué hace falta para resolver | Coste estimado |
| --- | --- | --- | --- |
| 1 — Evaluator aborta en predicciones inválidas | No podemos cerrar Phase 2 ni producir métricas oficiales de frontier. También bloqueará Phase 4-5 si el trained model emite cualquier acción inválida. | Cambiar el reader/evaluator para que una predicción inválida cuente como fila evaluada con `contract_valid=false`, no como error fatal del run completo. | Pequeño: Rust, ~1-2h + tests. |
| 2 — Predictor local sin constrained decoding | No podemos correr Phase 4 oficial con el modelo entrenado. | Opción A: añadir constrained decoding real (`outlines`/`xgrammar`) a `predict_operator_sft.py`. Opción B: desplegar un vLLM 0.5B con LoRA y usar una API OpenAI-compatible con guided decoding si está soportado. | A: ~2-3h. B: ~30-90 min si la infra vLLM ya soporta LoRA + schema; más si no. |
| 3 — 0.5B no está servido | No podemos usar `0.5b.llm.underpassai.com` para inferencia ahora mismo. | Crear un Deployment/Service real para Qwen 0.5B + LoRA, y apuntar el Ingress al Service correcto. | Medio, depende de vLLM/LoRA setup. |
| 4 — Gemma structured sigue parado | Puede haber impacto operativo fuera de este PR si alguien dependía de ese endpoint. | Decidir si se restaura Gemma structured o se reemplaza con otro backend. | Pequeño si solo es scale-up, pero hay que confirmar GPU availability. |

## 7. Lo que NO ha pasado pero el usuario podría asumir que pasó

- No hay un 0.5B sirviendo detrás de `0.5b.llm.underpassai.com`.
- No hay un `policy_eval_report.json` completo para v8.0.
- No se ha ejecutado Phase 4 trained prediction.
- No hay comparación frontier vs trained.
- No hay decisión Phase 5.
- No se aplicó el patch TensorBoard/step-eval antes del training.
- No hubo observer agent.
- No se hizo constrained decoding local.
- `--stop-after-json` no es constrained decoding.
- El frontier ceiling no está oficialmente scoreado; solo hay predicciones y diagnóstico no-oficial.
- El adapter sí existe y está completo, pero aún no está evaluado.
- El LoRA dir original `/tmp/operator-qwen05-lora` no se renombró; se copió a `/tmp/operator-qwen05-lora-v8.0` porque el original pertenece a `nobody` bajo `/tmp`.
- El archivo `docs/training/operator-v8-0-sft-closure-audit-2026-05-23.md` existe pero está sin commitear.
- `docs/training/viability_pack_gpt55.txt` sigue sin trackear.

## 8. Recomendación honesta sobre próximos pasos

Primero arreglaría el evaluator. Es el bloqueo más pequeño y desbloquea visibilidad para frontier y trained runs.

Recomendación concreta:

1. PR pequeño: `operator-policy-eval` debe contar predicciones inválidas como filas evaluadas, no abortar el run completo.
   - Mantener el error visible en detalles.
   - Métrica esperada: `contract_valid=false`.
   - El report debe incluir `invalid_prediction_count`.
   - Tests: una predicción inválida en medio de un archivo no aborta; se cuenta como inválida.

2. Después, re-ejecutar solo el scoring local contra el frontier `predictions.jsonl` ya existente. No hace falta pagar otra vez.

3. En paralelo o después, implementar un camino real de constrained decoding para el modelo entrenado.
   - Si queremos rapidez: usar vLLM/OpenAI-compatible con guided decoding, pero solo si podemos montar el LoRA y schema sin tocar demasiado infra.
   - Si queremos reproducibilidad local: añadir `outlines`/`xgrammar` a `predict_operator_sft.py`.

4. No usar `--stop-after-json` como cierre oficial. Lo aceptaría solo como diagnóstico informal si el usuario quiere una señal rápida del LoRA, claramente marcada como no comparable.

Mi lectura: no estamos ante "el training falló". Estamos ante "entrenamos antes de tener cerrado el arnés de evaluación real". El checkpoint existe; el trabajo pendiente es convertir las predicciones, válidas o inválidas, en métricas robustas y comparables.

