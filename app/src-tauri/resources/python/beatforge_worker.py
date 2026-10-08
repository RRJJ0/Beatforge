"""Beatforge production worker bootstrap.

Stdout is reserved for versioned JSON Lines protocol messages. All diagnostics,
including output produced while importing upstream modules, go to stderr.
"""

from __future__ import annotations

import contextlib
from datetime import datetime, timezone
import hashlib
import importlib
import json
import math
import os
from pathlib import Path
import platform
import signal
import sys
import threading
import time
import traceback
from typing import Any


PROTOCOL_VERSION = 1
WORKER_VERSION = "8"
MAX_MESSAGE_BYTES = 64 * 1024


def install_generated_group_guard() -> dict[str, int]:
    """Normalize unsafe generated group values before postprocessing.

    Mapperatorinator permits event groups to share the preceding TIME_SHIFT and
    interpolates untimed slider components. Passing no event_times to get_groups
    leaves those groups at the dataclass's 0 ms default, so resolve them with the
    upstream timing rules before checking their order. A non-positive generated
    slider velocity would otherwise produce a zero-length slider, so treat it as
    absent and let the upstream geometry fallback calculate a valid length.
    """
    from osuT5.osuT5.dataset import data_utils
    from osuT5.osuT5.inference import postprocessor

    original_get_groups = postprocessor.get_groups
    stats = {
        "defaulted_non_positive_slider_lengths": 0,
        "defaulted_non_positive_slider_svs": 0,
        "dropped_out_of_order_groups": 0,
    }

    original_timing_point_at = postprocessor.Postprocessor.timing_point_at
    original_calc_length = postprocessor.Postprocessor.calc_length
    slider_length_state = {"current_sv": 1.0}

    def tracked_timing_point_at(time, timing_points):
        timing_point = original_timing_point_at(time, timing_points)
        slider_length_state["current_sv"] = (
            1.0
            if timing_point.parent is None
            else -100 / timing_point.ms_per_beat
        )
        return timing_point

    def guarded_calc_length(
        self, sv, span_duration, redline, slider_multiplier
    ):
        length = original_calc_length(
            self,
            sv,
            span_duration,
            redline,
            slider_multiplier,
        )
        if math.isfinite(length) and length > 0:
            return length

        fallback_length = original_calc_length(
            self,
            slider_length_state["current_sv"],
            span_duration,
            redline,
            slider_multiplier,
        )
        if not math.isfinite(fallback_length) or fallback_length <= 0:
            raise RuntimeError(
                "Mapperatorinator could not calculate a positive slider length"
            )
        stats["defaulted_non_positive_slider_lengths"] += 1
        return fallback_length

    def guarded_get_groups(
        events: list[Any], *, event_times=None, types_first: bool = False
    ):
        resolved_event_times = event_times
        if resolved_event_times is None:
            resolved_event_times = []
            data_utils.update_event_times(
                events,
                resolved_event_times,
                types_first=types_first,
            )

        groups, group_indices = original_get_groups(
            events,
            event_times=resolved_event_times,
            types_first=types_first,
        )

        kept_groups = []
        kept_indices = []
        last_absolute_time: int | None = None
        for group, indices in zip(groups, group_indices):
            if (
                group.event_type == data_utils.EventType.SLIDER_HEAD
                and group.scroll_speed is not None
                and group.scroll_speed <= 0
            ):
                group.scroll_speed = None
                stats["defaulted_non_positive_slider_svs"] += 1
            if group.event_type in data_utils.TIMED_EVENTS and (
                last_absolute_time is not None
                and group.time < last_absolute_time
            ):
                stats["dropped_out_of_order_groups"] += 1
                continue
            kept_groups.append(group)
            kept_indices.append(indices)
            if group.event_type in data_utils.TIMED_EVENTS:
                last_absolute_time = group.time
        return kept_groups, kept_indices

    postprocessor.get_groups = guarded_get_groups
    postprocessor.Postprocessor.timing_point_at = staticmethod(
        tracked_timing_point_at
    )
    postprocessor.Postprocessor.calc_length = guarded_calc_length
    return stats


def reserve_protocol_stdout():
    """Keep one UTF-8 pipe for JSONL and send all other stdout to stderr.

    contextlib.redirect_stdout only replaces Python's sys.stdout object. Native
    extensions and background workers can still write locale-encoded bytes to
    file descriptor 1, corrupting the JSONL protocol. Duplicate the original
    pipe first, then redirect the process-level stdout descriptor before any
    third-party modules are imported.
    """
    protocol_fd = os.dup(sys.stdout.fileno())
    protocol_stdout = os.fdopen(
        protocol_fd,
        "w",
        encoding="utf-8",
        errors="strict",
        newline="\n",
        buffering=1,
    )
    os.dup2(sys.stderr.fileno(), sys.stdout.fileno())
    if os.name == "nt":
        # Some native Windows libraries bypass the C runtime descriptor and
        # resolve STD_OUTPUT_HANDLE directly. Point that handle at stderr too.
        import ctypes
        import msvcrt

        std_output_handle = 0xFFFF_FFF5
        stderr_handle = msvcrt.get_osfhandle(sys.stderr.fileno())
        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel32.SetStdHandle.argtypes = [ctypes.c_uint32, ctypes.c_void_p]
        kernel32.SetStdHandle.restype = ctypes.c_int
        if not kernel32.SetStdHandle(std_output_handle, stderr_handle):
            raise OSError("Failed to redirect the worker stdout handle")
    return protocol_stdout


PROTOCOL_STDOUT = reserve_protocol_stdout()
SEND_LOCK = threading.Lock()
CANCEL_REQUESTED = threading.Event()


def file_summary(path: Path) -> dict[str, Any]:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            digest.update(chunk)
    return {
        "name": path.name,
        "size_bytes": path.stat().st_size,
        "sha256": digest.hexdigest(),
    }


def reproduction_key(payload: dict[str, Any]) -> str:
    canonical = json.dumps(
        payload,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    return hashlib.sha256(canonical).hexdigest()


def json_value(value: Any) -> Any:
    if isinstance(value, (str, int, float, bool)) or value is None:
        return value
    if isinstance(value, (list, tuple)):
        return [json_value(item) for item in value]
    if isinstance(value, dict):
        return {str(key): json_value(item) for key, item in value.items()}
    if hasattr(value, "name"):
        return value.name
    return str(value)


def log(message: str) -> None:
    print(message, file=sys.stderr, flush=True)


def send(payload: dict[str, Any]) -> None:
    encoded = json.dumps(payload, ensure_ascii=False, separators=(",", ":"))
    if len(encoded.encode("utf-8")) > MAX_MESSAGE_BYTES:
        raise RuntimeError("Worker response exceeds the protocol message limit")
    with SEND_LOCK:
        PROTOCOL_STDOUT.write(encoded + "\n")
        PROTOCOL_STDOUT.flush()


def progress(job_id: str, stage: str, percent: int, message: str) -> None:
    send(
        {
            "type": "progress",
            "protocol_version": PROTOCOL_VERSION,
            "job_id": job_id,
            "stage": stage,
            "percent": percent,
            "message": message,
        }
    )


def classify_inference_error(error: Exception) -> tuple[str, str, bool]:
    messages: list[str] = []
    current: BaseException | None = error
    while current is not None and len(messages) < 8:
        messages.append(str(current))
        current = current.__cause__ or current.__context__
    detail = " ".join(messages).casefold()

    if "out of memory" in detail and ("cuda" in detail or "gpu" in detail):
        return (
            "CUDA_OUT_OF_MEMORY",
            "The GPU ran out of memory. Close other GPU applications and retry.",
            True,
        )
    if any(
        marker in detail
        for marker in (
            "driver version is insufficient",
            "no kernel image is available",
            "not compiled with cuda",
            "cuda initialization error",
            "cuda driver",
        )
    ):
        return (
            "CUDA_ENVIRONMENT_INCOMPATIBLE",
            "The installed NVIDIA driver and PyTorch CUDA runtime are incompatible. Run environment diagnostics.",
            False,
        )
    if "cuda error" in detail or "device-side assert" in detail:
        return (
            "CUDA_RUNTIME_FAILED",
            "GPU inference failed. Restart Beatforge before retrying; see the local job log for details.",
            True,
        )
    if any(
        marker in detail
        for marker in (
            "model_path is missing",
            "model.safetensors",
            "managed v32 model",
            "backbone configuration",
            "backbone identity",
        )
    ):
        return (
            "MODEL_UNAVAILABLE",
            "The managed Mapperatorinator model is missing or invalid. Verify or reinstall the model package.",
            False,
        )
    if "audio_path is missing or invalid" in detail:
        return (
            "AUDIO_INPUT_UNAVAILABLE",
            "The selected audio file is no longer available. Choose the file again and retry.",
            True,
        )
    if "reference_beatmap_path is missing or invalid" in detail:
        return (
            "REFERENCE_BEATMAP_UNAVAILABLE",
            "The selected reference beatmap is no longer available. Choose it again or generate without it.",
            True,
        )
    if any(
        marker in detail
        for marker in (
            "couldntdecodeerror",
            "couldn't decode",
            "error decoding",
            "invalid data found when processing input",
            "ffmpeg returned error",
            "audio_path has an unsupported",
        )
    ):
        return (
            "AUDIO_DECODE_FAILED",
            "The selected audio could not be decoded. Try converting it to WAV or FLAC and retry.",
            True,
        )
    if "missingconfigexception" in detail or "config search path" in detail:
        return (
            "INFERENCE_CONFIGURATION_FAILED",
            "The pinned Mapperatorinator configuration is incomplete. Repair the AI runtime.",
            False,
        )
    if any(
        marker in detail
        for marker in (
            "cpu dispatcher tracer already",
            "error importing numpy",
            "numpy.core.multiarray failed to import",
        )
    ):
        return (
            "RUNTIME_IMPORT_FAILED",
            "The managed Python runtime could not initialize NumPy or PyTorch. Restart Beatforge and retry; repair the AI environment if it happens again.",
            True,
        )
    return (
        "UPSTREAM_INFERENCE_FAILED",
        "Mapperatorinator could not finish this beatmap. See the local job log for technical details.",
        False,
    )


def watch_cancellation(job_id: str) -> None:
    # Avoid retaining sys.stdin's BufferedReader lock in a daemon thread during
    # interpreter shutdown.
    raw = bytearray()
    while len(raw) <= MAX_MESSAGE_BYTES + 1 and not raw.endswith(b"\n"):
        chunk = os.read(sys.stdin.fileno(), MAX_MESSAGE_BYTES + 2 - len(raw))
        if not chunk:
            return
        raw.extend(chunk)
    if not raw:
        return
    try:
        if len(raw) > MAX_MESSAGE_BYTES or not raw.endswith(b"\n"):
            raise ValueError("Cancellation command exceeds the protocol message limit")
        command = json.loads(raw)
        if (
            not isinstance(command, dict)
            or command.get("type") != "cancel_job"
            or command.get("protocol_version") != PROTOCOL_VERSION
            or command.get("job_id") != job_id
        ):
            raise ValueError("Invalid cancellation command")
        log(f"Cancellation requested for {job_id}")
        CANCEL_REQUESTED.set()
        signal.raise_signal(signal.SIGINT)
    except Exception as error:  # noqa: BLE001 - invalid control input is diagnostic only
        log(f"Cancellation watcher ignored input: {error}")


@contextlib.contextmanager
def heartbeat(job_id: str, stage: str, percent: int, message: str):
    stopped = threading.Event()

    def report() -> None:
        while not stopped.wait(15):
            try:
                progress(job_id, stage, percent, message)
            except Exception as error:  # noqa: BLE001 - main inference must own terminal state
                log(f"Progress heartbeat failed: {error}")
                return

    reporter = threading.Thread(target=report, name=f"beatforge-{stage}", daemon=True)
    reporter.start()
    try:
        yield
    finally:
        stopped.set()
        reporter.join(timeout=1)


@contextlib.contextmanager
def loading_step(job_id: str, stage: str, percent: int, message: str):
    progress(job_id, stage, percent, message)
    started = time.perf_counter()
    log(f"{stage} started")
    try:
        with heartbeat(job_id, stage, percent, message):
            yield
    except BaseException:
        log(f"{stage} interrupted after {time.perf_counter() - started:.3f}s")
        raise
    else:
        log(f"{stage} finished after {time.perf_counter() - started:.3f}s")


def read_command() -> dict[str, Any]:
    raw = sys.stdin.buffer.readline(MAX_MESSAGE_BYTES + 2)
    if not raw:
        raise ValueError("Worker command is missing")
    if len(raw) > MAX_MESSAGE_BYTES or not raw.endswith(b"\n"):
        raise ValueError("Worker command exceeds the protocol message limit")
    command = json.loads(raw)
    if not isinstance(command, dict):
        raise ValueError("Worker command must be a JSON object")
    if command.get("protocol_version") != PROTOCOL_VERSION:
        raise ValueError("Unsupported worker protocol version")
    return command


def normalized_path(value: Any) -> Path | None:
    if not isinstance(value, str) or not value:
        return None
    try:
        return Path(value).resolve(strict=True)
    except (OSError, RuntimeError):
        return None


def require_path(value: Any, label: str, *, directory: bool = False) -> Path:
    path = normalized_path(value)
    if path is None or (directory and not path.is_dir()) or (not directory and not path.is_file()):
        raise ValueError(f"{label} is missing or invalid")
    if not path.is_absolute():
        raise ValueError(f"{label} must be an absolute path")
    return path


def require_number(value: Any, label: str, minimum: float, maximum: float) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"{label} must be numeric")
    number = float(value)
    if not minimum <= number <= maximum:
        raise ValueError(f"{label} must be between {minimum} and {maximum}")
    return number


def require_descriptors(value: Any, label: str) -> list[str]:
    if not isinstance(value, list) or len(value) > 16:
        raise ValueError(f"{label} must be a list with at most 16 values")
    normalized: list[str] = []
    for item in value:
        if not isinstance(item, str) or not item.strip() or len(item.strip()) > 64:
            raise ValueError(f"{label} contains an invalid value")
        if any(character in item for character in ',[]"') or any(
            character.isspace() and character not in " \t" for character in item
        ):
            raise ValueError(f"{label} contains an unsupported character")
        normalized.append(item.strip())
    if len({item.casefold() for item in normalized}) != len(normalized):
        raise ValueError(f"{label} contains a duplicate value")
    return normalized


def install_local_v32_backbone_config(model_path: Path) -> None:
    """Prevent the pinned custom config from resolving Whisper metadata remotely.

    The V32 config.json already embeds the complete VarWhisper backbone config,
    but the pinned custom MapperatorinatorConfig constructor calls
    VarWhisperConfig.from_pretrained("openai/whisper-small") before applying it.
    """
    model_config = json.loads((model_path / "config.json").read_text("utf-8"))
    if model_config.get("backbone_model_name") != "OliBomby/varwhisper-small":
        raise RuntimeError("Managed V32 model has an unsupported backbone identity")
    backbone_config = model_config.get("backbone_config")
    if not isinstance(backbone_config, dict) or backbone_config.get("d_model") != 768:
        raise RuntimeError("Managed V32 model is missing its embedded backbone configuration")

    from osuT5.osuT5.model.custom_transformers.configuration_varwhisper import (
        VarWhisperConfig,
    )
    from transformers import WhisperConfig

    def from_embedded(cls, _pretrained_name: str, *args: Any, **kwargs: Any):
        del args, kwargs
        return cls.from_dict(dict(backbone_config))

    VarWhisperConfig.from_pretrained = classmethod(from_embedded)
    WhisperConfig.from_pretrained = classmethod(from_embedded)


def health_check(command: dict[str, Any]) -> int:
    errors: list[str] = []
    source_path = normalized_path(command.get("source_path"))
    model_path = normalized_path(command.get("model_path"))
    expected_source_commit = str(command.get("source_commit") or "")
    expected_model_version = str(command.get("model_version") or "")

    report: dict[str, Any] = {
        "type": "health",
        "protocol_version": PROTOCOL_VERSION,
        "worker_version": WORKER_VERSION,
        "ready": False,
        "python_version": platform.python_version(),
        "python_executable": sys.executable,
        "pytorch_version": None,
        "cuda_runtime": None,
        "cuda_available": False,
        "gpu_name": None,
        "ffmpeg_version": None,
        "ffmpeg_executable": None,
        "mapperatorinator_commit": expected_source_commit or None,
        "model_version": expected_model_version or None,
        "model_path": str(model_path) if model_path else None,
        "errors": errors,
    }

    try:
        with contextlib.redirect_stdout(sys.stderr):
            import torch

        report["pytorch_version"] = torch.__version__
        report["cuda_runtime"] = torch.version.cuda
        report["cuda_available"] = bool(torch.cuda.is_available())
        if report["cuda_available"]:
            report["gpu_name"] = torch.cuda.get_device_name(0)
    except Exception as error:  # noqa: BLE001 - health boundary must report all import failures
        errors.append(f"PyTorch: {error}")
        traceback.print_exc(file=sys.stderr)

    try:
        with contextlib.redirect_stdout(sys.stderr):
            import imageio_ffmpeg

        ffmpeg_executable = Path(imageio_ffmpeg.get_ffmpeg_exe()).resolve(strict=True)
        report["ffmpeg_version"] = imageio_ffmpeg.get_ffmpeg_version()
        report["ffmpeg_executable"] = str(ffmpeg_executable)
        os.environ["IMAGEIO_FFMPEG_EXE"] = str(ffmpeg_executable)
    except Exception as error:  # noqa: BLE001
        errors.append(f"FFmpeg: {error}")
        traceback.print_exc(file=sys.stderr)

    if source_path is None or not (source_path / "inference.py").is_file():
        errors.append("Mapperatorinator: managed source is missing")
    else:
        try:
            marker = json.loads((source_path / ".beatforge-install.json").read_text("utf-8"))
            if marker.get("sourceCommit") != expected_source_commit:
                raise RuntimeError("source marker does not match the requested commit")
            sys.path.insert(0, str(source_path))
            with contextlib.redirect_stdout(sys.stderr):
                inference = importlib.import_module("inference")
            for name in (
                "compile_args",
                "setup_inference_environment",
                "load_model_with_server",
                "get_config",
                "generate",
            ):
                if not callable(getattr(inference, name, None)):
                    raise RuntimeError(f"required inference API is missing: {name}")
        except Exception as error:  # noqa: BLE001
            errors.append(f"Mapperatorinator: {error}")
            traceback.print_exc(file=sys.stderr)

    required_model_files = (
        "model.safetensors",
        "config.json",
        "generation_config.json",
        "tokenizer.json",
    )
    if model_path is None:
        errors.append("Model: managed V32 model is not installed")
    else:
        missing = [name for name in required_model_files if not (model_path / name).is_file()]
        if missing:
            errors.append("Model: missing " + ", ".join(missing))

    report["ready"] = not errors
    send(report)
    return 0 if report["ready"] else 3


def run_inference(command: dict[str, Any], job_id: str) -> tuple[Path, dict[str, Any]]:
    """Execute the pinned inference API with a validated, local-only command."""
    job_id = command.get("job_id")
    if not isinstance(job_id, str) or not job_id:
        raise ValueError("start_job requires a job_id")
    source_path = require_path(command.get("source_path"), "source_path", directory=True)
    model_path = require_path(command.get("model_path"), "model_path", directory=True)
    audio_path = require_path(command.get("audio_path"), "audio_path")
    output_path = require_path(command.get("output_path"), "output_path", directory=True)
    if audio_path.suffix.lower() not in {".mp3", ".wav", ".ogg", ".m4a", ".flac"}:
        raise ValueError("audio_path has an unsupported file type")
    reference_value = command.get("reference_beatmap_path")
    if reference_value is not None:
        reference_path = require_path(reference_value, "reference_beatmap_path")
        if reference_path.suffix.lower() != ".osu":
            raise ValueError("reference_beatmap_path must point to an .osu file")

    started_at = datetime.now(timezone.utc)
    started = time.perf_counter()
    input_summary = {
        "audio": file_summary(audio_path),
        "reference_beatmap": file_summary(reference_path)
        if reference_value is not None
        else None,
    }

    if not (source_path / "inference.py").is_file() or not (
        source_path / ".beatforge-install.json"
    ).is_file():
        raise ValueError("source_path is not a managed Mapperatorinator installation")
    source_commit = command.get("source_commit")
    if not isinstance(source_commit, str) or not source_commit:
        raise ValueError("source_commit is missing")
    source_marker = json.loads((source_path / ".beatforge-install.json").read_text("utf-8"))
    if source_marker.get("sourceCommit") != source_commit:
        raise ValueError("source_path does not match source_commit")
    model_version = command.get("model_version")
    if not isinstance(model_version, str) or not model_version:
        raise ValueError("model_version is missing")
    missing_model_files = [
        name
        for name in ("model.safetensors", "config.json", "generation_config.json", "tokenizer.json")
        if not (model_path / name).is_file()
    ]
    if missing_model_files:
        raise ValueError("model_path is missing " + ", ".join(missing_model_files))

    config = command.get("config")
    if not isinstance(config, dict):
        raise ValueError("start_job requires a config object")
    if config.get("config_name") != "v32" or config.get("gamemode") != 0:
        raise ValueError("Only the pinned V32 osu!standard configuration is supported")
    require_number(config.get("difficulty"), "difficulty", 1.0, 8.0)
    seed = config.get("seed")
    if isinstance(seed, bool) or not isinstance(seed, int) or not 0 <= seed <= 65_535:
        raise ValueError("seed must be an integer between 0 and 65535")
    positive = require_descriptors(config.get("descriptors"), "descriptors")
    negative = require_descriptors(config.get("negative_descriptors"), "negative_descriptors")
    if {item.casefold() for item in positive} & {item.casefold() for item in negative}:
        raise ValueError("A descriptor cannot be both positive and negative")
    for field in ("title", "artist"):
        value = config.get(field)
        if not isinstance(value, str) or not value.strip() or len(value) > 200:
            raise ValueError(f"{field} is invalid")
    for field in ("approach_rate", "circle_size"):
        if config.get(field) is not None:
            require_number(config[field], field, 0.0, 10.0)
    if not isinstance(config.get("hitsounded"), bool):
        raise ValueError("hitsounded must be boolean")
    if config.get("device") not in {"cuda", "cpu", "auto"}:
        raise ValueError("Unsupported inference device")
    if config.get("precision") not in {"fp16", "bf16", "fp32"}:
        raise ValueError("Unsupported inference precision")

    progress(job_id, "adapter_ready", 3, "Validated the managed production request")
    os.environ["HF_HUB_OFFLINE"] = "1"
    os.environ["TRANSFORMERS_OFFLINE"] = "1"
    os.environ["TOKENIZERS_PARALLELISM"] = "false"
    os.chdir(source_path)
    sys.path.insert(0, str(source_path))

    # Initialize NumPy before PyTorch and before the cancellation watcher can
    # inject SIGINT into native module initialization. An interrupted NumPy
    # CPU dispatcher import can otherwise leave the process permanently stuck.
    with loading_step(job_id, "loading_numpy", 5, "Initializing the numerical runtime"):
        with contextlib.redirect_stdout(sys.stderr):
            import numpy  # noqa: F401 - explicit initialization boundary

    with loading_step(job_id, "loading_pytorch", 7, "Loading PyTorch and CUDA libraries"):
        with contextlib.redirect_stdout(sys.stderr):
            import torch

    with loading_step(job_id, "loading_audio_runtime", 8, "Loading the managed audio runtime"):
        with contextlib.redirect_stdout(sys.stderr):
            import imageio_ffmpeg
            from pydub import AudioSegment
            import pydub.audio_segment as pydub_audio_segment

            ffmpeg_executable = Path(imageio_ffmpeg.get_ffmpeg_exe()).resolve(strict=True)
            os.environ["IMAGEIO_FFMPEG_EXE"] = str(ffmpeg_executable)
            AudioSegment.converter = str(ffmpeg_executable)
            # imageio-ffmpeg intentionally ships no ffprobe. Pydub only uses probe
            # metadata to choose an explicit PCM codec; FFmpeg can safely select it.
            pydub_audio_segment.mediainfo_json = lambda *_args, **_kwargs: {}

    with loading_step(
        job_id,
        "loading_upstream",
        9,
        "Loading the pinned Mapperatorinator runtime",
    ):
        with contextlib.redirect_stdout(sys.stderr):
            inference = importlib.import_module("inference")
            generated_group_stats = install_generated_group_guard()

    with loading_step(job_id, "loading_inference_config", 10, "Loading inference configuration"):
        with contextlib.redirect_stdout(sys.stderr):
            install_local_v32_backbone_config(model_path)
            from hydra import compose, initialize_config_dir
            from omegaconf import OmegaConf

    # Runtime imports are complete, so cancellation can safely interrupt
    # configuration and inference without corrupting NumPy's native importer.
    threading.Thread(
        target=watch_cancellation,
        args=(job_id,),
        name=f"beatforge-cancel-{job_id}",
        daemon=True,
    ).start()

    progress(job_id, "configuring", 12, "Compiling the V32 inference configuration")
    with contextlib.redirect_stdout(sys.stderr):
        with initialize_config_dir(
            config_dir=str(source_path / "configs" / "inference"), version_base="1.1"
        ):
            args = OmegaConf.to_object(compose(config_name="v32"))

        args.model_path = str(model_path)
        args.auto_select_gamemode_model = False
        args.audio_path = str(audio_path)
        args.output_path = str(output_path)
        args.beatmap_path = str(reference_path) if reference_value is not None else ""
        args.gamemode = config["gamemode"]
        args.difficulty = config["difficulty"]
        args.seed = seed
        args.descriptors = positive
        args.negative_descriptors = negative
        args.title = config["title"].strip()
        args.title_unicode = args.title
        args.artist = config["artist"].strip()
        args.artist_unicode = args.artist
        args.creator = "Beatforge AI"
        args.version = f"AI {args.difficulty:.1f}★"
        args.approach_rate = config.get("approach_rate")
        args.circle_size = config.get("circle_size")
        args.hitsounded = config["hitsounded"]
        args.device = config["device"]
        # V32 is trained and shipped with bf16 as its CUDA precision. Older
        # Beatforge settings exposed fp16, but that can collapse the timing
        # logits into an invalid all-zero sampling distribution. Treat fp16 as
        # the legacy spelling for V32's bf16 mode; compile_args will safely
        # fall back to fp32 on GPUs without native bf16 support.
        args.precision = "bf16" if config["precision"] == "fp16" else config["precision"]
        args.lora_path = None
        args.add_to_beatmap = False
        args.overwrite_reference_beatmap = False
        args.export_osz = True
        args.use_server = False
        args.generate_positions = False
        # The pinned CUDA-graph decoder can pass an all-zero probability
        # distribution to torch.multinomial during V32 timing generation on
        # PyTorch 2.10. Use the upstream stock generation path for stability.
        args.fast_decoder_loop = False
        args.super_timing_fast_loop = False
        args.diff_ckpt = ""
        args.diff_refine_ckpt = ""
        if reference_value is not None:
            args.in_context = [inference.ContextType.TIMING]

        inference.compile_args(args, verbose=True)
        inference.setup_inference_environment(args.seed)
        if args.device == "cuda" and torch.cuda.is_available():
            torch.cuda.reset_peak_memory_stats()

    effective_parameters = {
        "config_name": "v32",
        "gamemode": args.gamemode,
        "difficulty": args.difficulty,
        "seed": args.seed,
        "descriptors": json_value(args.descriptors),
        "negative_descriptors": json_value(args.negative_descriptors),
        "title": args.title,
        "artist": args.artist,
        "approach_rate": args.approach_rate,
        "circle_size": args.circle_size,
        "hitsounded": args.hitsounded,
        "device": args.device,
        "precision": args.precision,
        "generate_positions": args.generate_positions,
        "export_osz": args.export_osz,
        "fast_decoder_loop": args.fast_decoder_loop,
        "super_timing_fast_loop": args.super_timing_fast_loop,
        "in_context": json_value(args.in_context),
        "output_type": json_value(args.output_type),
        "temperature": args.temperature,
        "timing_temperature": args.timing_temperature,
        "mania_column_temperature": args.mania_column_temperature,
        "taiko_hit_temperature": args.taiko_hit_temperature,
        "top_p": args.top_p,
        "top_k": args.top_k,
        "cfg_scale": args.cfg_scale,
        "num_beams": args.num_beams,
        "timeshift_bias": args.timeshift_bias,
    }

    progress(job_id, "loading_model", 20, "Loading the managed V32 model")
    with heartbeat(job_id, "loading_model", 20, "The V32 model is still loading"):
        with contextlib.redirect_stdout(sys.stderr):
            model, tokenizer = inference.load_model_with_server(
                args.model_path,
                args.train,
                args.device,
                max_batch_size=args.max_batch_size,
                use_server=False,
                precision=args.precision,
                attn_implementation=args.attn_implementation,
                lora_path=None,
                gamemode=args.gamemode,
                auto_select_gamemode_model=False,
                fast_decoder_loop=args.fast_decoder_loop,
            )
            generation_config, beatmap_config = inference.get_config(args)

    progress(job_id, "preparing_audio", 45, "Preparing audio and timing context")
    progress(job_id, "generating", 55, "Generating and post-processing the beatmap")
    with heartbeat(job_id, "generating", 55, "Beatmap inference is still running"):
        with contextlib.redirect_stdout(sys.stderr):
            _, result_path = inference.generate(
                args,
                generation_config=generation_config,
                beatmap_path=args.beatmap_path,
                beatmap_config=beatmap_config,
                model=model,
                tokenizer=tokenizer,
                timing_model=None,
                timing_tokenizer=None,
                diff_model=None,
                diff_tokenizer=None,
                refine_model=None,
                verbose=True,
            )

    defaulted_non_positive_slider_lengths = generated_group_stats[
        "defaulted_non_positive_slider_lengths"
    ]
    defaulted_non_positive_slider_svs = generated_group_stats[
        "defaulted_non_positive_slider_svs"
    ]
    dropped_out_of_order_groups = generated_group_stats[
        "dropped_out_of_order_groups"
    ]
    if defaulted_non_positive_slider_lengths:
        log(
            "Defaulted non-positive generated slider length calculations: "
            f"{defaulted_non_positive_slider_lengths}"
        )
    if defaulted_non_positive_slider_svs:
        log(
            "Defaulted non-positive generated slider velocity values: "
            f"{defaulted_non_positive_slider_svs}"
        )
    if dropped_out_of_order_groups:
        log(
            "Discarded generated event groups whose absolute time moved backwards: "
            f"{dropped_out_of_order_groups}"
        )

    progress(job_id, "validating_output", 95, "Checking the generated output path")
    result_path = Path(result_path).resolve(strict=True)
    try:
        result_path.relative_to(output_path)
    except ValueError as error:
        raise RuntimeError("Mapperatorinator returned an output outside the managed job directory") from error
    if result_path.suffix.lower() not in {".osu", ".osz"}:
        raise RuntimeError("Mapperatorinator returned an unsupported output file")
    completed_at = datetime.now(timezone.utc)
    gpu_active = args.device == "cuda" and torch.cuda.is_available()
    runtime = {
        "python_version": platform.python_version(),
        "pytorch_version": torch.__version__,
        "cuda_runtime": torch.version.cuda,
        "device": args.device,
        "precision": args.precision,
        "gpu_name": command.get("gpu_name")
        or (torch.cuda.get_device_name(0) if gpu_active else None),
        "gpu_memory_total_mb": command.get("gpu_memory_total_mb")
        or (
            round(torch.cuda.get_device_properties(0).total_memory / (1024 * 1024))
            if gpu_active
            else None
        ),
        "driver_version": command.get("driver_version"),
        "peak_gpu_memory_mb": round(torch.cuda.max_memory_allocated() / (1024 * 1024), 1)
        if gpu_active
        else None,
    }
    reproducible_inputs = {
        "source_commit": source_commit,
        "model_version": model_version,
        "worker_version": WORKER_VERSION,
        "parameters": effective_parameters,
        "input": input_summary,
    }
    provenance = {
        "schema_version": 1,
        "job_id": job_id,
        "reproduction_key": reproduction_key(reproducible_inputs),
        "source": {
            "mapperatorinator_commit": source_commit,
            "worker_version": WORKER_VERSION,
            "protocol_version": PROTOCOL_VERSION,
        },
        "model": {
            "package": "mapperatorinator-v32-standard",
            "version": model_version,
        },
        "runtime": runtime,
        "parameters": effective_parameters,
        "input": input_summary,
        "output": file_summary(result_path),
        "timing": {
            "started_at": started_at.isoformat(),
            "completed_at": completed_at.isoformat(),
            "elapsed_seconds": round(time.perf_counter() - started, 3),
        },
        "known_limitations": [
            "osu!standard only",
            "position diffusion disabled",
            "fast decoder disabled",
        ],
        "output_sanitization": {
            "defaulted_non_positive_slider_lengths": defaulted_non_positive_slider_lengths,
            "defaulted_non_positive_slider_svs": defaulted_non_positive_slider_svs,
            "dropped_out_of_order_groups": dropped_out_of_order_groups,
        },
    }
    return result_path, provenance


def start_job(command: dict[str, Any]) -> int:
    job_id = command.get("job_id")
    if not isinstance(job_id, str) or not job_id:
        raise ValueError("start_job requires a job_id")
    try:
        result_path, provenance = run_inference(command, job_id)
        defaulted_non_positive_slider_lengths = provenance["output_sanitization"][
            "defaulted_non_positive_slider_lengths"
        ]
        defaulted_non_positive_slider_svs = provenance["output_sanitization"][
            "defaulted_non_positive_slider_svs"
        ]
        dropped_out_of_order_groups = provenance["output_sanitization"][
            "dropped_out_of_order_groups"
        ]
        warnings = []
        if defaulted_non_positive_slider_lengths:
            warnings.append(
                "Defaulted "
                f"{defaulted_non_positive_slider_lengths} non-positive generated "
                "slider length calculation(s) to the current velocity baseline."
            )
        if defaulted_non_positive_slider_svs:
            warnings.append(
                "Defaulted "
                f"{defaulted_non_positive_slider_svs} non-positive generated "
                "slider velocity value(s) to geometry-based length calculation."
            )
        if dropped_out_of_order_groups:
            warnings.append(
                "Discarded "
                f"{dropped_out_of_order_groups} generated event group(s) "
                "whose absolute time moved backwards."
            )
        send(
            {
                "type": "completed",
                "protocol_version": PROTOCOL_VERSION,
                "job_id": job_id,
                "output_path": str(result_path),
                "warnings": warnings,
                "provenance": provenance,
            }
        )
        return 0
    except KeyboardInterrupt:
        log(f"Inference cancelled: {job_id}")
        send(
            {
                "type": "failed",
                "protocol_version": PROTOCOL_VERSION,
                "job_id": job_id,
                "code": "JOB_CANCELLED",
                "message": "Generation was cancelled.",
                "recoverable": True,
            }
        )
        return 130
    except Exception as error:  # noqa: BLE001 - worker boundary returns a stable protocol error
        if CANCEL_REQUESTED.is_set():
            log(f"Inference cancelled after interruption: {job_id}: {error}")
            send(
                {
                    "type": "failed",
                    "protocol_version": PROTOCOL_VERSION,
                    "job_id": job_id,
                    "code": "JOB_CANCELLED",
                    "message": "Generation was cancelled.",
                    "recoverable": True,
                }
            )
            return 130
        log(f"Inference failed: {error}")
        traceback.print_exc(file=sys.stderr)
        code, message, recoverable = classify_inference_error(error)
        send(
            {
                "type": "failed",
                "protocol_version": PROTOCOL_VERSION,
                "job_id": job_id,
                "code": code,
                "message": message,
                "recoverable": recoverable,
            }
        )
        return 5


def main() -> int:
    try:
        command = read_command()
        message_type = command.get("type")
        if message_type == "health_check":
            return health_check(command)
        if message_type == "start_job":
            return start_job(command)
        raise ValueError(f"Unsupported worker command: {message_type!r}")
    except Exception as error:  # noqa: BLE001
        log(f"Worker bootstrap failed: {error}")
        traceback.print_exc(file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
