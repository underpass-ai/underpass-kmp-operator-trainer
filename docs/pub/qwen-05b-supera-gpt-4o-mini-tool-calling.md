---
title: Cuando un modelo pequeño copia mejor los argumentos de una herramienta
published: false
description: Un caso empírico donde Qwen2.5-0.5B ajustado con LoRA supera a GPT-4o-mini en precisión de argumentos, no en razonamiento general.
tags: ia, llm, finetuning, tutorial
---

# Cuando un modelo pequeño copia mejor los argumentos de una herramienta

Un `Qwen2.5-0.5B-Instruct` ajustado con LoRA sobre un corpus curado de 1.622 llamadas a herramientas obtiene **74,38% de coincidencia exacta** en el conjunto de evaluación, frente al **37,19% de GPT-4o-mini**.

La diferencia no está en elegir la herramienta. GPT-4o-mini casi siempre elige bien. El problema está en los argumentos: referencias incompletas, timestamps inválidos y estructuras largas que no coinciden exactamente con lo esperado.

El modelo pequeño, una vez ajustado, aprende justo eso: copiar y producir estructuras estrictas con más disciplina.

Este artículo no defiende que "los modelos grandes no sirven" ni que "el fine-tuning lo arregla todo". El caso es más concreto:

> Cuando la tarea consiste en generar llamadas a herramientas con argumentos estrictos, un modelo pequeño ajustado puede superar a un modelo más grande que no está optimizado para copiar esos argumentos exactos.

## Objetivo: producir una acción JSON exacta

El modelo no tiene que mantener una conversación ni escribir texto libre. Debe producir una acción MCP en JSON.

El modelo debe hacer tres cosas:

1. elegir la herramienta correcta;
2. rellenar los argumentos exactos;
3. detenerse o escalar cuando corresponde.

Las herramientas son doce. Algunas son sencillas, como `kernel_wake` o `kernel_inspect`. Otras exigen argumentos anidados, por ejemplo `kernel_ingest`, con dimensiones, entradas, relaciones, evidencia, procedencia e idempotencia.

Para generar esa acción, cada ejemplo contiene:

- un objetivo en lenguaje natural;
- un estado visible: referencias disponibles, presupuesto restante y herramientas permitidas;
- una acción esperada con herramienta y argumentos.

El punto importante es este: en una llamada a herramienta, una salida aproximada no sirve. Si el sistema que ejecuta la acción espera una referencia concreta, una fecha RFC3339 o un cursor de un tipo determinado, una paráfrasis tampoco sirve.

La comparación se basa en tres métricas:

- **Herramienta correcta**: el modelo elige el mismo tipo de acción o la misma herramienta que esperamos.
- **Contrato válido**: la acción se puede parsear y cumple las reglas mínimas del operador.
- **Coincidencia exacta**: la herramienta y los argumentos coinciden con la acción esperada.

La coincidencia exacta es la métrica dura. Una acción puede usar la herramienta correcta y seguir siendo inútil si el argumento está mal.

## Los números

El corpus final tiene 1.622 filas. De ellas, 242 quedan reservadas para evaluación y no se usan durante el ajuste.

La comparación usa dos modelos sobre esas mismas 242 filas:

- `gpt-4o-mini`, a temperatura 0.0;
- `Qwen2.5-0.5B-Instruct` ajustado con LoRA.

GPT-4o-mini tiene dos papeles en el proceso. Primero actúa como modelo profesor para generar candidatos de corpus. Después se usa como modelo grande de referencia sobre el conjunto de evaluación. Para evitar circularidad, solo entran al corpus las acciones que pasan validación estricta.

| Métrica | GPT-4o-mini | Qwen 0.5B + LoRA | Diferencia |
|---|---:|---:|---:|
| Predicciones evaluadas | 242 | 242 | - |
| Salidas con forma inválida | 12 / 242 (4,96%) | 0 / 242 (0,00%) | -12 |
| Coincidencia exacta | 90 / 242 (37,19%) | 180 / 242 (74,38%) | +37,19 pp |
| Herramienta correcta | 230 / 242 (95,04%) | 242 / 242 (100,00%) | +4,96 pp |
| Contrato válido | 220 / 242 (90,91%) | 242 / 242 (100,00%) | +9,09 pp |

El entrenamiento tarda unos 12 minutos en 4 GPUs Ampere. No hay coste de API durante el entrenamiento.

## Lo que GPT-4o-mini hace mal

La métrica global de GPT-4o-mini no parece mala a primera vista:

- 95,04% de selección correcta de herramienta;
- 90,91% de acciones válidas según contrato.

Pero al desglosar por herramienta aparece el patrón:

| Herramienta | Coincidencia exacta de GPT-4o-mini |
|---|---:|
| `kernel_wake` | 58 / 58 (100%) |
| `kernel_inspect` | 14 / 15 (93%) |
| `kernel_goto` | 3 / 3 (100%) |
| `kernel_ask` | 9 / 30 (30%) |
| `kernel_near` | 0 / 18 (0%) |
| `kernel_rewind` | 0 / 13 (0%) |
| `kernel_forward` | 0 / 35 (0%) |
| `kernel_trace` | 0 / 23 (0%) |
| `kernel_ingest` | 0 / 19 (0%) |
| `kernel_write_memory` | 0 / 22 (0%) |

En muchas de esas filas, GPT-4o-mini elige la herramienta correcta. Falla al generar los argumentos exactos.

El ejemplo más claro es `kernel_ingest`: 12 predicciones contienen `observed_at: "..."`, un placeholder en vez de un timestamp válido. No es un error de intención; es un error de contrato.

GPT-4o-mini sabe qué quiere hacer. No produce la llamada exacta que el sistema necesita.

## Lo que aprende el modelo ajustado

Con el mismo conjunto de evaluación, el Qwen de 0.5B da este resultado:

| Herramienta | GPT-4o-mini | Qwen 0.5B + LoRA | Diferencia |
|---|---:|---:|---:|
| `kernel_wake` | 58 / 58 (100%) | 58 / 58 (100%) | empate |
| `kernel_inspect` | 14 / 15 (93%) | 15 / 15 (100%) | +1 |
| `kernel_goto` | 3 / 3 (100%) | 3 / 3 (100%) | empate |
| `kernel_ask` | 9 / 30 (30%) | 13 / 30 (43%) | +4 |
| `kernel_near` | 0 / 18 (0%) | 15 / 18 (83%) | +15 |
| `kernel_rewind` | 0 / 13 (0%) | 13 / 13 (100%) | +13 |
| `kernel_forward` | 0 / 35 (0%) | 33 / 35 (94%) | +33 |
| `kernel_trace` | 0 / 23 (0%) | 23 / 23 (100%) | +23 |
| stop / escalate | 6 / 6 (100%) | 6 / 6 (100%) | empate |
| `kernel_ingest` | 0 / 19 (0%) | 0 / 19 (0%) | empate |
| `kernel_write_memory` | 0 / 22 (0%) | 1 / 22 (5%) | +1 |

El 0.5B no pierde en ninguna herramienta. Empata en las fáciles y mejora de forma clara en las que exigen copiar cursores, referencias y estructuras concretas.

El resultado más importante no es solo el `+37,19 pp` en coincidencia exacta. Es que el modelo ajustado produce **100% de acciones válidas según contrato**.

Eso no significa que resuelva todos los payloads largos. Las herramientas de escritura, especialmente `kernel_ingest`, siguen siendo el límite. La mejora principal aparece en llamadas donde la estructura es estricta, pero el argumento exacto puede copiarse desde el estado visible.

## Cómo se entrena

El modelo base es `Qwen/Qwen2.5-0.5B-Instruct`. El ajuste usa LoRA con un preset estándar para SFT:

```python
SFTConfig(
    learning_rate=2e-4,
    lr_scheduler_type="cosine",
    warmup_ratio=0.03,
    num_train_epochs=3,
    per_device_train_batch_size=4,
    gradient_accumulation_steps=1,  # 4 GPUs x 4 x 1 = batch efectivo 16
    max_length=2048,
    bf16=True,
    packing=False,
)
```

Detalles del run:

- LoRA `r=16`, `alpha=32`, `dropout=0,05`;
- módulos objetivo: `q_proj,k_proj,v_proj,o_proj,gate_proj,up_proj,down_proj`;
- 4 GPUs Ampere, DDP con `torchrun --nproc_per_node=4`;
- 258 steps;
- unos 12 minutos de reloj.

La curva es estable:

```text
epoch 1: eval_loss=0.0391, token_acc=0.9870
epoch 2: eval_loss=0.0242, token_acc=0.9899
epoch 3: eval_loss=0.0235, token_acc=0.9900
```

El entrenamiento termina sin NaNs, sin errores de memoria y sin señales de inestabilidad.

## Por qué puede pasar esto

Mi lectura es que aquí chocan dos objetivos de entrenamiento distintos.

Los modelos grandes están optimizados para ser útiles en lenguaje natural. Eso suele implicar:

- reformular;
- completar;
- explicar;
- convertir una entrada rígida en una respuesta más cómoda para una persona.

En muchas tareas eso es una virtud. En llamadas a herramientas con contrato estricto, no.

Si el estado visible contiene esta referencia:

```text
about:incident:checkout-errors:kernel_inspect:after-near:case-000:node:hypothesis:000
```

el sistema no quiere una referencia parecida. Quiere exactamente esos bytes.

Lo mismo ocurre con fechas, claves de idempotencia, cursores temporales o relaciones entre nodos. Una salida "mejor redactada" puede ser inválida.

El SFT de un modelo pequeño cambia el incentivo. El objetivo ya no es sonar útil, sino predecir el siguiente token exacto del ejemplo. La paráfrasis queda penalizada. Copiar bien queda recompensado.

En esta tarea concreta, esa literalidad vale más que la capacidad general del modelo grande.

## Por qué el resultado es medible

La parte menos vistosa del trabajo es la más importante: hacer que el resultado sea fiable.

Al principio hay un pipeline mucho más rápido, principalmente en Python. Genera corpus, llama al modelo profesor, valida algo de formato y produce estadísticas.

El problema es que algunos resultados marcados como correctos no lo son.

Un ejemplo: una plantilla espera `Stop(reason=NoCandidate)`, pero el gate acepta `Stop(reason=AnswerReady)` porque solo mira el tipo de acción: "es un stop". No comprueba el motivo concreto.

Otro ejemplo: una plantilla espera un cursor temporal, pero el gate acepta un cursor por referencia porque la herramienta sigue siendo `kernel_goto`.

La prueba corta marca verde. La semántica está mal.

Ahí el pipeline rápido deja de ser una fuente fiable. Sin esa limpieza, el `+37` sería un número bonito pero poco fiable.

## Qué sostiene la medición

No es un refactor por estética. Es un refactor para saber qué es verdad.

El sistema queda dividido en varios contextos:

```text
shared      -> value objects y contrato común
synthetic   -> generación y filtrado de corpus
evaluation  -> evaluación contra la respuesta esperada
replay      -> ejecución contra el sistema real
training    -> preparación SFT, entrenamiento y predicción
```

Las reglas más importantes son:

1. **Una clase por archivo.**
   Evita que validación, formato de intercambio, evaluación y generación se mezclen en funciones enormes.

2. **Nada de strings sueltos para herramientas, cursores o modos.**
   `kernel_trace`, `writer_pre_read` o `cursor.kind=time` viven como value objects tipados con `parse` y `as_str`.

3. **Nada de `serde_json` en dominio ni aplicación.**
   El JSON pertenece a infraestructura. El dominio trabaja con tipos.

4. **Nada de fallbacks silenciosos.**
   Si una acción no parsea o viola contrato, se registra como fallo. No se repara a escondidas.

5. **Nada de "abrir" acciones aceptadas para que suba el número.**
   Si el modelo falla, el modelo falla. Si el dataset está mal, se corrige el dataset.

Gracias a eso podemos distinguir tres cosas que antes se mezclaban:

- herramienta incorrecta;
- herramienta correcta con argumentos incorrectos;
- salida inválida según contrato.

## Cómo se limpia el corpus

Cada fila generada por el modelo profesor pasa por validadores estrictos.

El contrato comprueba, entre otras cosas:

- si la herramienta está permitida en el modo actual;
- si queda presupuesto para hacer una llamada;
- si las referencias usadas existen en el estado visible;
- si los argumentos de escritura tienen la forma esperada;
- si la acción puede ejecutarse de forma coherente.

Además, algunos escenarios tienen criterios semánticos propios. Por ejemplo:

- si la acción es `stop`, el `reason` esperado debe coincidir;
- si la acción es `kernel_goto`, el tipo de cursor esperado debe coincidir.

Eso cierra el agujero de "herramienta correcta, semántica equivocada".

También quedan fuera plantillas que no pertenecen a este corpus. En particular, varias plantillas de `escalate` y `stop:no-candidate` miden una preferencia de política: "en este contexto deberías escalar" o "deberías parar aunque todavía puedas preguntar".

El contrato actual no exige esas acciones. Si el contrato no las exige, no deben estar en un corpus que pretende medir cumplimiento estricto de contrato.

Esas plantillas quedan en backlog para una futura especificación prescriptiva. El corpus final pasa de 1.650 a 1.622 escenarios.

## Límites del resultado

Este resultado es útil, pero no conviene exagerarlo.

### `kernel_ingest` sigue sin resolverse

`kernel_ingest` es la herramienta más difícil del conjunto. Requiere argumentos largos con objetos relacionados entre sí: dimensiones, entradas, evidencia, relaciones y procedencia.

Los dos modelos obtienen 0% de coincidencia exacta en `kernel_ingest`.

La diferencia es que GPT-4o-mini produce varias salidas inválidas, mientras que el Qwen ajustado produce acciones válidas. Aun así, los argumentos no coinciden exactamente con la respuesta esperada.

Esto queda para v8.1. Puede requerir mejor evaluación semántica de argumentos, más diversidad en ejemplos, resolución determinista de argumentos preparados o generación restringida por esquema.

### Sin generación restringida por esquema

Las predicciones se generan como JSON libre, sin `outlines`, `xgrammar` ni `json_schema strict`.

Esto hace que la comparación sea justa entre ambos modelos, porque los dos se evalúan sin esas ayudas. Pero la metodología más limpia debería repetir el experimento con generación restringida por esquema en ambos lados.

### La evaluación es dentro de la misma distribución

Las 242 filas de evaluación salen del mismo corpus que el entrenamiento, separadas por `about`. No son plantillas completamente nuevas ni dominios desconocidos.

Por tanto, este resultado mide aprendizaje dentro de la distribución del corpus. No mide todavía generalización fuerte a escenarios nuevos.

### El modelo de referencia es GPT-4o-mini

La comparación no incluye GPT-4o completo, Claude Opus u otros modelos tier-1. En una prueba pequeña, varios modelos GPT-5.x no mejoran a GPT-4o-mini a temperatura determinista en los casos que nos importan.

Por eso GPT-4o-mini queda como modelo profesor y como referencia de comparación.

### No es una receta para tareas abiertas

Este resultado no aplica directamente a tareas de juicio abierto, explicación larga o razonamiento no acotado. Aquí el espacio de salida estaba muy restringido: herramienta, argumentos y contrato.

## Qué me parece accionable

Si estás construyendo agentes que llaman a herramientas con esquemas estrictos, no asumas que el modelo más grande será el mejor generando llamadas exactas.

La receta que funciona aquí es:

1. usar un modelo razonable como profesor;
2. validar cada salida con contrato estricto;
3. descartar ejemplos ambiguos o prescriptivos que el contrato no respalda;
4. ajustar un modelo pequeño sobre el corpus limpio;
5. medir coincidencia exacta de argumentos, no solo selección de herramienta.

Con 1.622 ejemplos y 12 minutos de entrenamiento obtenemos un modelo que:

- genera 100% de acciones válidas;
- supera al modelo profesor por 37 puntos en coincidencia exacta;
- cuesta cero en API durante inferencia local.

Eso no significa que el modelo pequeño sea mejor para razonar, escribir o conversar. Significa que, en una tarea acotada y tipada, con un corpus limpio, puede ser mejor copiando la estructura exacta que necesita el sistema.

## Recursos

Artículos relacionados:

- **APIGen** — [Liu et al., 2024](https://arxiv.org/abs/2406.18518): verificación multi-etapa para corpus de uso de herramientas.
- **xLAM** — [Zhang et al., 2024](https://arxiv.org/abs/2409.03215): modelos pequeños superando a GPT-3.5 y Claude Haiku en BFCL.
- **Hammer** — [Lin et al., 2024](https://arxiv.org/abs/2410.04587): function masking y detección de herramientas irrelevantes.
- **ToolACE** — [Liu et al., 2025 ICLR](https://openreview.net/forum?id=8EB8k6DdCU): generación de corpus con self-evolution.
- **Octopus v2** — [Chen & Li, 2024](https://arxiv.org/abs/2404.01744): functional tokens para selección de herramienta en un solo forward.
- **Small Models, Big Tasks** — [Lu et al., 2025](https://arxiv.org/abs/2504.19277): estudio empírico de modelos pequeños en llamadas a herramientas.

## Cierre

Este es el primer modelo de referencia entrenado del que podemos sacar conclusiones interpretables.

No dice "el modelo está listo para producción". Dice algo más limitado y más útil: en un benchmark controlado, dentro de la misma distribución, de llamadas a herramientas con contrato estricto, un Qwen de 0.5B ajustado supera a GPT-4o-mini en exactitud de argumentos.

GPT-4o-mini no falla porque elija mal las herramientas. Falla porque no copia los argumentos con la precisión que exige el sistema.

El modelo pequeño cierra buena parte de ese hueco.

La lección no es "usa siempre modelos pequeños". La lección es: si tu tarea depende de estructuras exactas, mide estructuras exactas. Y si quieres entrenar un modelo para eso, antes necesitas un corpus que no mienta.
