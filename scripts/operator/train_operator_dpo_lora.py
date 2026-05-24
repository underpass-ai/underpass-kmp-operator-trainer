#!/usr/bin/env python3
"""Train a DPO LoRA adapter on operator chosen/rejected action pairs."""

from __future__ import annotations

import argparse
import hashlib
import inspect
import json
import statistics
from pathlib import Path
from typing import Any

from predict_operator_sft import validate_action_shape


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Train operator LoRA with DPO pairs.")
    parser.add_argument("--base-adapter-dir", required=True, type=Path)
    parser.add_argument("--pairs-jsonl", required=True, type=Path)
    parser.add_argument("--model-id", default="Qwen/Qwen2.5-0.5B-Instruct")
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--trust-remote-code", action="store_true")
    parser.add_argument(
        "--device-map",
        choices=["auto", "none"],
        default="none",
        help="Use 'none' for torchrun/DDP so Accelerate owns device placement.",
    )
    parser.add_argument(
        "--torch-dtype",
        choices=["auto", "float16", "bfloat16", "float32"],
        default="auto",
    )
    parser.add_argument("--learning-rate", type=float, default=5e-6)
    parser.add_argument("--beta", type=float, default=0.1)
    parser.add_argument("--epochs", type=float, default=2.0)
    parser.add_argument("--batch-size", type=int, default=2)
    parser.add_argument("--grad-accum", type=int, default=4)
    parser.add_argument("--max-length", type=int, default=4096)
    parser.add_argument("--max-prompt-length", type=int, default=3072)
    parser.add_argument("--bf16", action="store_true")
    parser.add_argument("--fp16", action="store_true")
    parser.add_argument(
        "--validate-only",
        action="store_true",
        help="Validate DPO JSONL contract and exit before importing training dependencies.",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    rows = read_jsonl(args.pairs_jsonl)
    validate_pairs(rows)
    if args.validate_only:
        print(
            json.dumps(
                {
                    "event": "kernel_operator_dpo_train.validate_only",
                    "pairs": len(rows),
                    "status": "ok",
                },
                indent=2,
                sort_keys=True,
            )
        )
        return

    try:
        import torch
        from datasets import Dataset
        from peft import PeftModel
        from transformers import AutoModelForCausalLM, AutoTokenizer
        from trl import DPOConfig, DPOTrainer
    except ImportError as exc:
        raise SystemExit(
            "Missing DPO training dependencies. Install torch, transformers, "
            "datasets, peft, accelerate, trl, and tensorboard."
        ) from exc

    tokenizer = AutoTokenizer.from_pretrained(
        args.model_id,
        use_fast=True,
        trust_remote_code=args.trust_remote_code,
    )
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token

    dpo_rows = [transform_pair_to_dpo_format(row, tokenizer) for row in rows]
    length_report = compute_token_length_report(dpo_rows, tokenizer)
    print(json.dumps(length_report, indent=2, sort_keys=True))
    if length_report["max_prompt_tokens"] > args.max_prompt_length:
        raise SystemExit(
            "DPO prompt token length exceeds --max-prompt-length: "
            f"{length_report['max_prompt_tokens']} > {args.max_prompt_length}"
        )
    if length_report["max_total_tokens"] > args.max_length:
        raise SystemExit(
            "DPO pair token length exceeds --max-length: "
            f"{length_report['max_total_tokens']} > {args.max_length}"
        )

    dataset = Dataset.from_list(dpo_rows)
    splits = dataset.train_test_split(test_size=0.1, seed=42)

    model_kwargs: dict[str, Any] = {
        "torch_dtype": torch_dtype(args.torch_dtype, args.bf16, args.fp16, torch),
        "trust_remote_code": args.trust_remote_code,
    }
    if args.device_map != "none":
        model_kwargs["device_map"] = args.device_map
    base_model = AutoModelForCausalLM.from_pretrained(args.model_id, **model_kwargs)
    model = PeftModel.from_pretrained(
        base_model,
        str(args.base_adapter_dir),
        is_trainable=True,
    )

    trainer = DPOTrainer(
        model=model,
        args=build_dpo_config(args, DPOConfig),
        train_dataset=splits["train"],
        eval_dataset=splits["test"],
        processing_class=tokenizer,
    )
    trainer.train()
    trainer.save_model(str(args.output_dir))
    if trainer.is_world_process_zero():
        tokenizer.save_pretrained(str(args.output_dir))
        write_summary(args, len(rows), length_report, trainer.state.log_history)


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open(encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            if not line.strip():
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError as exc:
                raise SystemExit(f"{path}:{line_number}: invalid JSONL row") from exc
            if not isinstance(row, dict):
                raise SystemExit(f"{path}:{line_number}: row must be an object")
            rows.append(row)
    if not rows:
        raise SystemExit(f"{path}: DPO pairs file must not be empty")
    return rows


def validate_pairs(rows: list[dict[str, Any]]) -> None:
    seen_keys: set[str] = set()
    for index, row in enumerate(rows, start=1):
        scenario_id = require_string(row, "scenario_id", index)
        step_id = require_string(row, "step_id", index)
        prompt_messages = row.get("prompt_messages")
        if not isinstance(prompt_messages, list) or len(prompt_messages) != 2:
            raise SystemExit(f"pair {index}: prompt_messages must contain system+user")
        roles = [
            message.get("role") for message in prompt_messages if isinstance(message, dict)
        ]
        if roles != ["system", "user"]:
            raise SystemExit(f"pair {index}: expected system/user prompt roles, got {roles}")
        if not all(isinstance(message.get("content"), str) for message in prompt_messages):
            raise SystemExit(f"pair {index}: prompt message content must be strings")

        chosen = require_object(row, "chosen", index)
        rejected = require_object(row, "rejected", index)
        if canonical_json(chosen) == canonical_json(rejected):
            raise SystemExit(f"pair {index}: chosen and rejected are identical")

        chosen_shape_error = validate_action_shape(chosen)
        if chosen_shape_error is not None:
            raise SystemExit(
                f"pair {index}: chosen action violates strict contract: "
                f"{chosen_shape_error}"
            )

        perturbation = require_object(row, "perturbation", index)
        require_string(perturbation, "name", index)
        require_string(perturbation, "field", index)

        violation_codes = row.get("rejected_violation_codes")
        if not isinstance(violation_codes, list) or not all(
            isinstance(item, str) and item for item in violation_codes
        ):
            raise SystemExit(
                f"pair {index}: rejected_violation_codes must be non-empty strings"
            )
        if not violation_codes:
            raise SystemExit(f"pair {index}: rejected_violation_codes must not be empty")

        unique_key = canonical_json(
            {
                "scenario_id": scenario_id,
                "step_id": step_id,
                "chosen": chosen,
                "rejected": rejected,
                "perturbation": perturbation,
            }
        )
        if unique_key in seen_keys:
            raise SystemExit(f"pair {index}: duplicate DPO pair")
        seen_keys.add(unique_key)


def transform_pair_to_dpo_format(
    row: dict[str, Any],
    tokenizer: Any,
) -> dict[str, str]:
    prompt_messages = row["prompt_messages"]
    prompt = apply_chat_template(tokenizer, prompt_messages)
    return {
        "prompt": prompt,
        "chosen": action_response_text(row["chosen"]),
        "rejected": action_response_text(row["rejected"]),
    }


def apply_chat_template(tokenizer: Any, messages: list[dict[str, str]]) -> str:
    if getattr(tokenizer, "chat_template", None):
        return tokenizer.apply_chat_template(
            messages,
            tokenize=False,
            add_generation_prompt=True,
        )
    return "".join(
        f"<|im_start|>{message['role']}\n{message['content']}<|im_end|>\n"
        for message in messages
    ) + "<|im_start|>assistant\n"


def action_response_text(action: dict[str, Any]) -> str:
    return canonical_json({"action": action})


def compute_token_length_report(
    rows: list[dict[str, str]],
    tokenizer: Any,
) -> dict[str, Any]:
    prompt_lengths: list[int] = []
    total_lengths: list[int] = []
    for row in rows:
        prompt_tokens = token_count(tokenizer, row["prompt"])
        chosen_total = token_count(tokenizer, row["prompt"] + row["chosen"])
        rejected_total = token_count(tokenizer, row["prompt"] + row["rejected"])
        prompt_lengths.append(prompt_tokens)
        total_lengths.append(max(chosen_total, rejected_total))
    return {
        "event": "kernel_operator_dpo_train.token_lengths",
        "pairs": len(rows),
        "max_prompt_tokens": max(prompt_lengths),
        "p99_prompt_tokens": percentile(prompt_lengths, 99),
        "max_total_tokens": max(total_lengths),
        "p99_total_tokens": percentile(total_lengths, 99),
        "p90_total_tokens": percentile(total_lengths, 90),
    }


def token_count(tokenizer: Any, text: str) -> int:
    return len(tokenizer(text, add_special_tokens=False)["input_ids"])


def percentile(values: list[int], percent: int) -> int:
    if not values:
        return 0
    if len(values) == 1:
        return values[0]
    quantiles = statistics.quantiles(values, n=100, method="inclusive")
    return int(quantiles[percent - 1])


def require_string(row: dict[str, Any], key: str, index: int) -> str:
    value = row.get(key)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"pair {index}: {key} must be a non-empty string")
    return value


def require_object(row: dict[str, Any], key: str, index: int) -> dict[str, Any]:
    value = row.get(key)
    if not isinstance(value, dict):
        raise SystemExit(f"pair {index}: {key} must be an object")
    return value


def canonical_json(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"), sort_keys=True)


def torch_dtype(value: str, bf16: bool, fp16: bool, torch: Any) -> str | Any:
    if bf16 and fp16:
        raise SystemExit("--bf16 and --fp16 are mutually exclusive")
    if bf16:
        return torch.bfloat16
    if fp16:
        return torch.float16
    if value == "auto":
        return "auto"
    return {
        "float16": torch.float16,
        "bfloat16": torch.bfloat16,
        "float32": torch.float32,
    }[value]


def build_dpo_config(args: argparse.Namespace, dpo_config_cls: Any) -> Any:
    kwargs: dict[str, Any] = {
        "output_dir": str(args.output_dir),
        "num_train_epochs": args.epochs,
        "learning_rate": args.learning_rate,
        "lr_scheduler_type": "cosine",
        "warmup_ratio": 0.1,
        "per_device_train_batch_size": args.batch_size,
        "per_device_eval_batch_size": args.batch_size,
        "gradient_accumulation_steps": args.grad_accum,
        "max_length": args.max_length,
        "beta": args.beta,
        "bf16": args.bf16,
        "fp16": args.fp16,
        "logging_steps": 10,
        "logging_dir": str(args.output_dir / "tensorboard"),
        "eval_strategy": "steps",
        "eval_steps": 20,
        "save_strategy": "epoch",
        "report_to": "tensorboard",
        "ddp_find_unused_parameters": False,
    }
    supported = inspect.signature(dpo_config_cls.__init__).parameters
    if "max_prompt_length" in supported:
        kwargs["max_prompt_length"] = args.max_prompt_length
    else:
        print(
            json.dumps(
                {
                    "event": "kernel_operator_dpo_train.max_prompt_length_not_supported",
                    "max_prompt_length": args.max_prompt_length,
                    "note": "prompt length was validated before trainer construction",
                },
                sort_keys=True,
            )
        )
    missing = sorted(
        key for key in ["max_length", "beta", "learning_rate"] if key not in supported
    )
    if missing:
        raise SystemExit(
            "DPOConfig does not support required arguments: " + ", ".join(missing)
        )
    return dpo_config_cls(**kwargs)


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_summary(
    args: argparse.Namespace,
    pair_count: int,
    length_report: dict[str, Any],
    log_history: list[dict[str, Any]],
) -> None:
    args.output_dir.mkdir(parents=True, exist_ok=True)
    summary = {
        "event": "kernel_operator_dpo_train.completed",
        "pairs_jsonl": str(args.pairs_jsonl),
        "pairs_sha256": file_sha256(args.pairs_jsonl),
        "pair_count": pair_count,
        "base_adapter_dir": str(args.base_adapter_dir),
        "model_id": args.model_id,
        "output_dir": str(args.output_dir),
        "hyperparameters": {
            "learning_rate": args.learning_rate,
            "beta": args.beta,
            "epochs": args.epochs,
            "batch_size": args.batch_size,
            "grad_accum": args.grad_accum,
            "max_length": args.max_length,
            "max_prompt_length": args.max_prompt_length,
            "bf16": args.bf16,
            "fp16": args.fp16,
        },
        "token_lengths": length_report,
        "last_log_entries": log_history[-20:],
    }
    summary_path = args.output_dir / "dpo_training_summary.json"
    summary_path.write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
