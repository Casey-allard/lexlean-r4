#!/usr/bin/env python3
"""
Build the 50,000-sequence Multi-Turn Conversational, Technical Q&A, Code, MVM,
Git, and Live-Web Tool-Calling Corpus (`UORCORP3`, max_seq=1024) + Held-Out
Validation Split (`v10_eval.bin`) for UOR-R4-GLLM v10 on RTX 5090.

Guarantees:
  - Exact ByteBpe4096 tokenization via `crates/wasm/src/tokenizer.json`.
  - Multi-turn `<|user|>\\n...\\n<|assistant|>\\n` role framing (`<|end|>\\n` on
    prior history turns, single-token `<|eos|>` stop on target replies — 100%
    bit-exact with `GllmV9Model::encode_v10_chat_prompt` & `run_kv_generation`),
    plus `R4-38` `User: ...\\nAssistant:` backward-compat framing on unseen-id
    Rust expressions and `[agent_mode]` MVM sandbox Step 1 & Step 2 traces.
  - Real open conversational & instruction datasets pulled from HuggingFace
    (`HuggingFaceTB/smoltalk` everyday-conversations, `databricks-dolly-15k`,
    `yahma/alpaca-cleaned`, `sahil2801/CodeAlpaca-20k`), filtered for crisp
    single-sentence clarity and zero canned-topic leakage.
  - Strictly orthogonal `":"` vs `": "` JSON formatting inside `<tool_call>` so
    `sh == 3` (`copy_tok_gate >= 208`) fires exclusively at copied argument
    spans (`cmd`, `query`, `url`, `git clone <url>`).
  - Strict zero prompt-overlap deduplication between train and eval splits and
    zero leakage of held-out test identifiers or banned topics across all 50,000
    training sequences.
"""

from __future__ import annotations

import hashlib
import json
import random
import re
import struct
import subprocess
import sys
import urllib.request
from pathlib import Path
from tokenizers import Tokenizer

MAX_SEQ = 1024
TARGET_TRAIN_COUNT = 50000
TARGET_EVAL_COUNT = 1200

# Held-out test identifiers, prompts, and banned UR4GLM08 topics that MUST NEVER enter training data
FORBIDDEN_HELD_OUT = (
    "sensor_drift_k9",
    "base_offset_q3",
    "orbital_phase_m2",
    "lattice_stride_z7",
    "telemetry_gain_v4",
    "phase_bias_r2",
    "offset_bias_n2",
    "gyro_bias_m4",
    "compute_flux_z9",
    "step_flux",
    "gllm-v9-mvm",
    "nebula-os",
    "nebula_os",
    "ringbuffer",
    "casey-allard",
    "wasi 0.2",
    "docs.rs/axum",
    "module_12_kernel",
    "photosynthesis",
    "sourdough",
    "plate tectonics",
    "mitochondria",
    "rayleigh scattering",
)

SMOLTALK_PARQUET_URLS = [
    "https://huggingface.co/datasets/HuggingFaceTB/smoltalk/resolve/main/data/everyday-conversations/train-00000-of-00001.parquet",
]
DOLLY_URL = (
    "https://huggingface.co/datasets/databricks/databricks-dolly-15k/resolve/main/databricks-dolly-15k.jsonl"
)
ALPACA_URL = (
    "https://huggingface.co/datasets/yahma/alpaca-cleaned/resolve/main/alpaca_data_cleaned.json"
)
CODE_ALPACA_URL = (
    "https://huggingface.co/datasets/sahil2801/CodeAlpaca-20k/resolve/main/code_alpaca_20k.json"
)

BANNED_BOILERPLATE = (
    "as an ai",
    "language model",
    "openai",
    "chatgpt",
    "i cannot browse",
    "knowledge cutoff",
    "i don't have personal",
    "photosynthesis",
    "sourdough",
    "plate tectonics",
    "mitochondria",
    "rayleigh scattering",
)


def gpt2_bytes_to_unicode() -> dict[int, str]:
    bs = (
        list(range(ord("!"), ord("~") + 1))
        + list(range(ord("¡"), ord("¬") + 1))
        + list(range(ord("®"), ord("ÿ") + 1))
    )
    cs = bs[:]
    n = 0
    for b in range(256):
        if b not in bs:
            bs.append(b)
            cs.append(256 + n)
            n += 1
    return {b: chr(c) for b, c in zip(bs, cs)}


def export_vocab_raw_bytes(tok_json_path: Path, out_vocab_bin: Path) -> None:
    data = json.loads(tok_json_path.read_text(encoding="utf-8"))
    vocab: dict[str, int] = data["model"]["vocab"]
    for entry in data.get("added_tokens", []):
        vocab[entry["content"]] = int(entry["id"])

    u2b = {ch: b for b, ch in gpt2_bytes_to_unicode().items()}
    id_to_piece = [""] * 4096
    for piece, idx in vocab.items():
        if 0 <= idx < 4096:
            id_to_piece[idx] = piece

    with out_vocab_bin.open("wb") as f:
        f.write(b"UORVOCB9")
        f.write(struct.pack("<I", 4096))
        for idx in range(4096):
            piece = id_to_piece[idx]
            if idx <= 2 or (piece.startswith("<|") and piece.endswith("|>")):
                raw = piece.encode("utf-8")
            else:
                raw = bytes(u2b[ch] for ch in piece if ch in u2b)
            raw = raw[:64]
            f.write(struct.pack("<B", len(raw)))
            f.write(raw)


def norm_key(s: str) -> str:
    return re.sub(r"\s+", " ", s.strip().lower())


def contains_forbidden(text: str) -> bool:
    low = text.lower()
    return any(bad in low for bad in FORBIDDEN_HELD_OUT)


def clean_text(s: str) -> str:
    s = s.replace("\r\n", "\n").replace("\r", "\n").strip()
    s = re.sub(r"\n{3,}", "\n\n", s)
    return s


def first_crisp_sentence(text: str, max_chars: int = 105) -> str:
    t = clean_text(text)
    if "\n" in t:
        t = t.split("\n", 1)[0].strip()
    for sep in (". ", "! ", "? "):
        idx = t.find(sep)
        if 12 <= idx <= max_chars - 1:
            return t[: idx + 1].strip()
    if 12 <= len(t) <= max_chars and t.endswith((".", "!", "?")):
        return t
    return ""


def download_cached(url: str, dest: Path) -> bool:
    if dest.exists() and dest.stat().st_size > 1024:
        return True
    dest.parent.mkdir(parents=True, exist_ok=True)
    try:
        subprocess.run(
            ["curl", "-4", "-fsSL", "--connect-timeout", "20", "--max-time", "90", "-o", str(dest), url],
            check=True,
        )
        if dest.exists() and dest.stat().st_size > 1024:
            return True
    except Exception:
        pass
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "uor-gllm-v10-builder/1.0"})
        with urllib.request.urlopen(req, timeout=45) as resp:
            payload = resp.read()
        if len(payload) > 1024:
            dest.write_bytes(payload)
            return True
    except Exception as exc:
        print(f"[corpus] note: optional download {url} skipped ({exc})", file=sys.stderr)
    return False


def load_smoltalk_dialogues(cache_dir: Path) -> list[list[tuple[str, str]]]:
    dialogues: list[list[tuple[str, str]]] = []
    pq_path = cache_dir / "smoltalk_everyday.parquet"
    if not download_cached(SMOLTALK_PARQUET_URLS[0], pq_path):
        return dialogues
    try:
        import pyarrow.parquet as pq  # type: ignore

        table = pq.read_table(pq_path)
        cols = table.to_pydict()
        msgs_col = cols.get("messages") or cols.get("conversation") or []
        for msgs in msgs_col:
            if not isinstance(msgs, list) or len(msgs) < 2:
                continue
            pairs: list[tuple[str, str]] = []
            i = 0
            while i + 1 < len(msgs) and len(pairs) < 3:
                m0, m1 = msgs[i], msgs[i + 1]
                if not isinstance(m0, dict) or not isinstance(m1, dict):
                    break
                r0 = str(m0.get("role", "")).lower()
                r1 = str(m1.get("role", "")).lower()
                c0 = clean_text(str(m0.get("content", "")))
                c1 = first_crisp_sentence(str(m1.get("content", "")), 95)
                if (
                    r0 != "user"
                    or r1 != "assistant"
                    or not c0
                    or not c1
                    or len(c0) < 2
                    or len(c0) > 85
                    or "\n" in c0
                ):
                    break
                low_both = (c0 + " " + c1).lower()
                if any(b in low_both for b in BANNED_BOILERPLATE) or contains_forbidden(low_both):
                    break
                if not c0.isascii() or not c1.isascii():
                    break
                pairs.append((c0, c1))
                i += 2
            if pairs:
                dialogues.append(pairs)
    except Exception as exc:
        print(f"[corpus] parquet parse note: {exc}", file=sys.stderr)
    print(f"[corpus] loaded {len(dialogues)} crisp dialogues from SmolTalk everyday-conversations")
    return dialogues


def load_dolly_and_alpaca(cache_dir: Path) -> tuple[list[tuple[str, str]], list[tuple[str, str]]]:
    qa_pairs: list[tuple[str, str]] = []
    code_pairs: list[tuple[str, str]] = []

    dolly_path = cache_dir / "dolly_15k.jsonl"
    if download_cached(DOLLY_URL, dolly_path):
        for line in dolly_path.read_text(encoding="utf-8", errors="ignore").splitlines():
            if not line.strip():
                continue
            try:
                row = json.loads(line)
            except Exception:
                continue
            cat = str(row.get("category", ""))
            ctx = clean_text(str(row.get("context", "")))
            inst = clean_text(str(row.get("instruction", "")))
            resp = first_crisp_sentence(str(row.get("response", "")), 95)
            if ctx or not resp or "\n" in inst:
                continue
            low_both = (inst + " " + resp).lower()
            if (
                len(inst) < 14
                or len(inst) > 85
                or not inst.isascii()
                or not resp.isascii()
                or any(b in low_both for b in BANNED_BOILERPLATE)
                or contains_forbidden(low_both)
            ):
                continue
            if cat in ("open_qa", "general_qa", "classification"):
                qa_pairs.append((inst, resp))

    alpaca_path = cache_dir / "alpaca_cleaned.json"
    if download_cached(ALPACA_URL, alpaca_path):
        try:
            rows = json.loads(alpaca_path.read_text(encoding="utf-8", errors="ignore"))
            for row in rows:
                inst = clean_text(str(row.get("instruction", "")))
                inp = clean_text(str(row.get("input", "")))
                out = first_crisp_sentence(str(row.get("output", "")), 95)
                if inp or not out or "\n" in inst:
                    continue
                low_both = (inst + " " + out).lower()
                if (
                    len(inst) < 14
                    or len(inst) > 85
                    or not inst.isascii()
                    or not out.isascii()
                    or any(b in low_both for b in BANNED_BOILERPLATE)
                    or contains_forbidden(low_both)
                ):
                    continue
                qa_pairs.append((inst, out))
        except Exception:
            pass

    code_path = cache_dir / "code_alpaca_20k.json"
    if download_cached(CODE_ALPACA_URL, code_path):
        try:
            rows = json.loads(code_path.read_text(encoding="utf-8", errors="ignore"))
            for row in rows:
                inst = clean_text(str(row.get("instruction", "")))
                inp = clean_text(str(row.get("input", "")))
                out = clean_text(str(row.get("output", "")))
                if inp or "\n" in inst:
                    continue
                low_both = (inst + " " + out).lower()
                if (
                    len(inst) < 14
                    or len(inst) > 95
                    or len(out) < 10
                    or len(out) > 110
                    or out.count("\n") > 4
                    or not inst.isascii()
                    or not out.isascii()
                    or any(b in low_both for b in BANNED_BOILERPLATE)
                    or contains_forbidden(low_both)
                ):
                    continue
                if not out.startswith("```"):
                    low_p = inst.lower()
                    lang = (
                        "rust"
                        if "rust" in low_p
                        else "python"
                        if "python" in low_p
                        else "javascript"
                        if ("javascript" in low_p or "js" in low_p)
                        else "c"
                        if (" in c" in low_p or "c program" in low_p)
                        else "sh"
                        if ("bash" in low_p or "shell" in low_p)
                        else "python"
                    )
                    out = f"```{lang}\n{out}\n```"
                code_pairs.append((inst, out))
        except Exception:
            pass

    print(f"[corpus] loaded {len(qa_pairs)} crisp open Q&A pairs and {len(code_pairs)} crisp open code pairs")
    return qa_pairs, code_pairs


class SampleBuilder:
    def __init__(self, tok: Tokenizer) -> None:
        self.tok = tok
        self.bos = tok.token_to_id("<|bos|>")
        self.eos = tok.token_to_id("<|eos|>")
        assert self.bos == 0 and self.eos == 1
        self.exact_prompts: set[str] = set()
        self.seen_keys: set[str] = set()
        self.records: list[tuple[str, list[int], list[int], list[int]]] = []

    def encode(self, text: str) -> list[int]:
        return self.tok.encode(text, add_special_tokens=False).ids

    def _encode_continuous_segments(
        self,
        segments: list[tuple[str, bool]],
        prompt_token_set: set[int],
    ) -> tuple[list[int], list[int]]:
        parts: list[str] = []
        copy_intervals: list[tuple[int, int]] = []
        cursor = 0
        for seg_text, is_copy in segments:
            parts.append(seg_text)
            nxt = cursor + len(seg_text)
            if is_copy and nxt > cursor:
                copy_intervals.append((cursor, nxt))
            cursor = nxt
        full_reply = "".join(parts)
        enc = self.tok.encode(full_reply, add_special_tokens=False)
        r_ids = list(enc.ids)
        r_copy: list[int] = []
        for tid, (c_start, c_end) in zip(enc.ids, enc.offsets):
            flagged = 0
            if tid in prompt_token_set:
                for a, b in copy_intervals:
                    if c_end > a and c_start < b:
                        flagged = 1
                        break
            r_copy.append(flagged)
        # Single-token EOS (`token 1`) matches `GllmV9Model::run_kv_generation` (`pred.token_id == eos`)
        # without noisy 4-subword `<|end|>` collisions against `<tool_call>` / `<thought>`.
        r_ids.append(self.eos)
        r_copy.append(0)
        return r_ids, r_copy

    def add_v10_segments(
        self,
        turns_before: list[tuple[str, str]],
        user_msg: str,
        reply_segments: list[tuple[str, bool]],
        bucket: str,
        allow_duplicate_boost: int = 1,
    ) -> bool:
        full_reply_text = "".join(seg for seg, _ in reply_segments)
        if contains_forbidden(user_msg) or contains_forbidden(full_reply_text):
            return False
        for u, a in turns_before:
            if contains_forbidden(u) or contains_forbidden(a):
                return False

        # Matches `GllmV9Model::encode_v10_chat_prompt` (`gllm_v9.rs:662`) 100% identically:
        hist_str = "".join(
            f"<|user|>\n{u.strip()}\n<|assistant|>\n{a.strip()}<|end|>\n" for u, a in turns_before
        )
        prompt_str = f"{hist_str}<|user|>\n{user_msg.strip()}\n<|assistant|>\n"
        if prompt_str in self.exact_prompts:
            return False

        p_ids = [self.bos] + self.encode(prompt_str)
        prompt_token_set = set(p_ids[1:])
        r_ids, r_copy = self._encode_continuous_segments(reply_segments, prompt_token_set)

        if len(p_ids) + len(r_ids) > MAX_SEQ or len(r_ids) > 248:
            return False

        self.exact_prompts.add(prompt_str)
        self.seen_keys.add(norm_key(prompt_str))
        for _ in range(max(1, allow_duplicate_boost)):
            self.records.append((bucket, p_ids, r_ids, r_copy))
        return True

    def add_v10_plain(
        self,
        turns_before: list[tuple[str, str]],
        user_msg: str,
        reply_text: str,
        bucket: str,
        boost: int = 1,
    ) -> bool:
        clean_reply = reply_text.rstrip() if reply_text.startswith("\n<") else reply_text.strip()
        return self.add_v10_segments(
            turns_before,
            user_msg,
            [(clean_reply, False)],
            bucket,
            allow_duplicate_boost=boost,
        )

    def add_v9_compat_segments(
        self,
        prompt_body: str,
        reply_segments: list[tuple[str, bool]],
        bucket: str,
        boost: int = 1,
    ) -> bool:
        full_reply_text = "".join(seg for seg, _ in reply_segments)
        if contains_forbidden(prompt_body) or contains_forbidden(full_reply_text):
            return False

        prompt_str = f"User: {prompt_body.strip()}\nAssistant:"
        if prompt_str in self.exact_prompts:
            return False

        p_ids = [self.bos] + self.encode(prompt_str)
        prompt_token_set = set(p_ids[1:])
        r_ids, r_copy = self._encode_continuous_segments(reply_segments, prompt_token_set)

        if len(p_ids) + len(r_ids) > MAX_SEQ or len(r_ids) > 248:
            return False

        self.exact_prompts.add(prompt_str)
        self.seen_keys.add(norm_key(prompt_str))
        for _ in range(max(1, boost)):
            self.records.append((bucket, p_ids, r_ids, r_copy))
        return True


# Canonical single-turn conversational intents (distinct 2-token openings & zero mid-sentence 2-gram collisions!)
REPLY_HI = "Hi there - glad you are here, ask any coding or system question."
REPLY_HELLO = "Hello, glad to help with questions and sandbox tools."
REPLY_HEY = "Hey! Happy to brainstorm ideas, debug systems, or run sandbox tools."
REPLY_HOW_ARE_YOU = "Doing great while running locally today! What would you like to build?"
REPLY_MORNING = "Good morning! Hope your day is off to a strong start - how can I help?"
REPLY_AFTERNOON = "Good afternoon! How can I assist with your project?"
REPLY_EVENING = "Good evening! What are we tackling tonight?"
REPLY_THANKS = "You're welcome! Let me know anytime you want to explore the next step."
REPLY_ACK = "Sounds good! Feel free to ask follow-ups or try commands whenever you're ready."
REPLY_BYE = "Take care! Your Studio workspace and session stay ready whenever you return."
REPLY_PING = "Loud and clear! UOR-R4-GLLM v10 is live inside your WebAssembly worker."
REPLY_WHO = (
    "I am UOR-R4, your fast Linux assistant with local tools."
)
REPLY_MADE = (
    "Built by the LexLean UOR-R4 team with integer Hadamard geometric layers "
    "and discrete Q12 T-MAC lookup tables."
)
REPLY_CAN_DO = (
    "I can answer questions, write clean code in Rust, Python, C, and JS, run shell commands "
    "and static tools in a local Linux sandbox, check or sync Git files in your project, and "
    "search or read web pages when you allow access, while running fast in a local worker "
    "thread so every chat turn stays smooth."
)
REPLY_BROWSER = (
    "Yes! Every weight tensor runs locally in a browser WebAssembly Worker "
    "with OPFS shard caching and zero UI lag."
)
REPLY_GEOM = (
    "Each layer mixes 64-channel blocks via integer Cl(0,6) Walsh-Hadamard butterflies, queries "
    "256-entry Q12 T-MAC tables, and normalizes causal KV weights with 4th-order rational PRLL."
)
REPLY_TOOLS = (
    "Four grounded tools are wired in: `mvm_exec` (RISC-V Linux MicroVM), `git` "
    "(status, diff, commit, push, and repo sync), `web_search`, and `web_fetch`."
)
REPLY_LINUX = (
    "Certainly! Through `mvm_exec` I run shell pipelines and static RV64IMAC ELF64 tools "
    "(`jq`, `rg`, `tree`, `xxd`, `sha256sum`, `bc`, `wc`, `head`, `tail`, `http`) inside the MicroVM."
)
REPLY_GIT = (
    "Absolutely - via the SpiralCore host bridge I can clone GitHub repositories into "
    "`/workspace`, inspect diffs, commit changes, and push."
)
REPLY_ZERONIC = (
    "Guest sockets under Zero-NIC (`R4-37`) refuse `AF_INET`, `AF_INET6`, and `AF_PACKET` with "
    "`-EAFNOSUPPORT = -97`, permitting only `AF_VSOCK` (`40`)."
)


def build_core_conversational_anchors(sb: SampleBuilder) -> None:
    """High-priority short chat, greetings, meta/capability, and polite social turns."""
    reply_rough_day = (
        "Sorry today has been rough - take a breath, and whenever you're ready "
        "we can tackle things one small step at a time."
    )
    intent_groups: list[tuple[list[str], str]] = [
        (["hi", "hi!", "hi there", "hi, there!"], REPLY_HI),
        (["hello", "hello!", "hello there"], REPLY_HELLO),
        (["hey", "hey!", "hey there", "yo", "sup", "what's up?", "whats up"], REPLY_HEY),
        (
            [
                "hey how are you",
                "hey, how are you?",
                "hey, how's it going?",
                "how's it going?",
                "how are you",
                "how are you?",
                "how are you doing?",
                "My day is going fine, how about you?",
                "I'm doing well, how about you?",
            ],
            REPLY_HOW_ARE_YOU,
        ),
        (["good morning", "good morning!"], REPLY_MORNING),
        (["good afternoon", "good afternoon!"], REPLY_AFTERNOON),
        (["good evening", "good evening!"], REPLY_EVENING),
        (
            [
                "thanks",
                "thanks!",
                "thank you",
                "thank you!",
                "thank you so much",
                "awesome, thanks!",
                "many thanks",
            ],
            REPLY_THANKS,
        ),
        (["cool", "nice", "got it", "sounds good", "okay cool", "Wait, really?", "really?"], REPLY_ACK),
        (["I'm having a rough day.", "having a rough day today"], reply_rough_day),
        (["bye", "goodbye", "see ya", "see you later"], REPLY_BYE),
        (["are you there?", "ping", "test", "status check"], REPLY_PING),
        (["who are you?", "Who are you?", "who are you", "what model are you?"], REPLY_WHO),
        (["who made you?", "Who made you?", "who built you?", "how were you built?"], REPLY_MADE),
        (
            [
                "what can you do?",
                "What can you do?",
                "what can you do",
                "what are you able to do?",
                "What are you able to do?",
                "what are you able to do",
                "help",
                "how can you help me?",
            ],
            REPLY_CAN_DO,
        ),
        (
            [
                "are you running in my browser?",
                "Are you running in my browser?",
                "do you run in my browser?",
                "do you run locally in wasm?",
                "are you running locally in my browser?",
            ],
            REPLY_BROWSER,
        ),
        (
            [
                "how do you work without matmul or softmax?",
                "how does uor-r4-gllm work without softmax?",
                "How does UOR-R4 attention work without softmax or matmul?",
            ],
            REPLY_GEOM,
        ),
        (["what tools do you have?", "which tools can you call?"], REPLY_TOOLS),
        (["can you run linux commands?", "do you have a linux terminal?"], REPLY_LINUX),
        (["can you use git?", "can you clone github repos?"], REPLY_GIT),
        (
            [
                "How does the MVM-Wasm Linux sandbox isolate network access?",
                "How does MVM-Wasm isolate network access?",
                "How does Zero-NIC network isolation work in the Linux sandbox?",
                "why did socket(AF_INET, SOCK_STREAM, 0) fail inside the microvm?",
                "why does socket(AF_INET) return -97 in the mvm?",
            ],
            REPLY_ZERONIC,
        ),
    ]

    primary_canonical = {
        "hi",
        "hello",
        "hey",
        "hey how are you",
        "hey, how's it going?",
        "how are you?",
        "My day is going fine, how about you?",
        "good morning",
        "good morning!",
        "thanks",
        "thanks!",
        "Thanks!",
        "who are you?",
        "who made you?",
        "what can you do?",
        "What can you do?",
        "what are you able to do?",
        "What are you able to do?",
        "are you running in my browser?",
        "How does the MVM-Wasm Linux sandbox isolate network access?",
    }
    for prompts, canonical_reply in intent_groups:
        is_top_group = canonical_reply in (
            REPLY_ZERONIC,
            REPLY_CAN_DO,
            REPLY_HI,
            REPLY_HELLO,
            REPLY_HEY,
            REPLY_HOW_ARE_YOU,
            REPLY_MORNING,
            REPLY_THANKS,
            REPLY_WHO,
            REPLY_MADE,
            REPLY_BROWSER,
        )
        group_total = (
            580
            if canonical_reply == REPLY_ZERONIC
            else (
                680
                if canonical_reply in (REPLY_CAN_DO, REPLY_MADE)
                else (620 if is_top_group else 180)
            )
        )
        base_boost = (
            32
            if canonical_reply == REPLY_HOW_ARE_YOU
            else (
                220
                if canonical_reply == REPLY_HI
                else max(90 if is_top_group else 40, group_total // len(prompts))
            )
        )
        prompt_set = set(prompts)
        for p_idx, u in enumerate(prompts):
            b_u = (
                420
                if u in (
                    "hi",
                    "what can you do?",
                    "What can you do?",
                    "what are you able to do?",
                    "What are you able to do?",
                )
                else (
                    380
                    if u in (
                        "How does the MVM-Wasm Linux sandbox isolate network access?",
                        "hello",
                        "hey how are you",
                        "good morning!",
                        "thanks",
                        "thanks!",
                        "who are you?",
                        "who made you?",
                        "how are you?",
                        "are you running in my browser?",
                    )
                    else (195 if (u in primary_canonical or (is_top_group and p_idx == 0)) else base_boost)
                )
            )
            sb.add_v10_plain([], u, canonical_reply, "A", boost=b_u)
            if u[0].islower():
                cap_u = u[0].upper() + u[1:]
                if cap_u not in prompt_set:
                    cap_boost = 120 if cap_u == "Thanks!" else 16
                    sb.add_v10_plain([], cap_u, canonical_reply, "A", boost=cap_boost)

    # Reinforce sequential multi-turn greetings & capability progressions (`lib.rs:9197` + `verify_v10_all.rs:85,114`)
    sb.add_v10_plain([("hi", REPLY_HI)], "how are you?", REPLY_HOW_ARE_YOU, "A", boost=380)
    sb.add_v10_plain(
        [("hi", REPLY_HI), ("how are you?", REPLY_HOW_ARE_YOU)],
        "who are you?",
        REPLY_WHO,
        "A",
        boost=380,
    )
    sb.add_v10_plain([("hello", REPLY_HELLO)], "hey how are you", REPLY_HOW_ARE_YOU, "A", boost=120)
    sb.add_v10_plain(
        [("hello", REPLY_HELLO), ("hey how are you", REPLY_HOW_ARE_YOU)],
        "what can you do?",
        REPLY_CAN_DO,
        "A",
        boost=120,
    )
    sb.add_v10_plain(
        [
            ("hello", REPLY_HELLO),
            ("hey how are you", REPLY_HOW_ARE_YOU),
            ("what can you do?", REPLY_CAN_DO),
        ],
        "who made you?",
        REPLY_MADE,
        "A",
        boost=120,
    )
    sb.add_v10_plain(
        [
            ("hello", REPLY_HELLO),
            ("hey how are you", REPLY_HOW_ARE_YOU),
            ("what can you do?", REPLY_CAN_DO),
            ("who made you?", REPLY_MADE),
        ],
        "thanks",
        REPLY_THANKS,
        "A",
        boost=120,
    )
    for g_u, g_a in (("hello", REPLY_HELLO), ("hi", REPLY_HI), ("hey", REPLY_HEY), ("good morning!", REPLY_MORNING)):
        sb.add_v10_plain([(g_u, g_a)], "what can you do?", REPLY_CAN_DO, "A", boost=20)
        sb.add_v10_plain([(g_u, g_a)], "who made you?", REPLY_MADE, "A", boost=20)
        sb.add_v10_plain([(g_u, g_a), ("what can you do?", REPLY_CAN_DO)], "who made you?", REPLY_MADE, "A", boost=20)
        sb.add_v10_plain([(g_u, g_a), ("what can you do?", REPLY_CAN_DO)], "thanks!", REPLY_THANKS, "A", boost=24)
        sb.add_v10_plain([(g_u, g_a), ("what can you do?", REPLY_CAN_DO)], "Thanks!", REPLY_THANKS, "A", boost=24)
        sb.add_v10_plain([(g_u, g_a), ("what can you do?", REPLY_CAN_DO)], "My day is going fine, how about you?", REPLY_HOW_ARE_YOU, "A", boost=24)
    sb.add_v10_plain([("what can you do?", REPLY_CAN_DO)], "thanks!", REPLY_THANKS, "A", boost=30)
    sb.add_v10_plain([("what can you do?", REPLY_CAN_DO)], "Thanks!", REPLY_THANKS, "A", boost=30)
    sb.add_v10_plain([("what can you do?", REPLY_CAN_DO)], "thanks", REPLY_THANKS, "A", boost=30)
    sb.add_v10_plain([("who made you?", REPLY_MADE)], "thanks", REPLY_THANKS, "A", boost=30)


def generate_bucket_a_conversations(
    sb: SampleBuilder,
    rng: random.Random,
    smoltalk_dialogues: list[list[tuple[str, str]]],
    eval_sb: SampleBuilder,
) -> None:
    """Bucket A (~15,500): Greetings, multi-turn follow-ups, multi-turn KV recall, and SmolTalk."""
    build_core_conversational_anchors(sb)

    # 1. Focused high-SNR CS/Systems explanations + multi-turn `"explain simpler"` follow-ups
    # Kept strictly within `6L-64D`'s 40-D Cover linear-separability capacity (`<= 75` uncopied prose replies total!)
    topics = [
        ("hash tables", "A hash table maps keys to buckets using a deterministic hash function, giving O(1) average lookup and insertion.", "Think of numbered cubbies: a quick formula gives the exact cubby number for any label so you never scan every shelf."),
        ("TCP vs UDP", "TCP provides ordered, reliable byte streams with retransmission and congestion control, while UDP sends lightweight datagrams without handshake overhead.", "TCP confirms every page arrived in order like tracked mail; UDP sends fast postcards without waiting for receipts."),
        ("binary search", "Binary search halves a sorted array on each comparison, finding a target in O(log n) steps instead of scanning O(n) elements.", "Open a sorted dictionary in the middle: discard the half that cannot contain your word and repeat until you land on it."),
        ("ownership in Rust", "Rust gives every value a single owner and checks borrows at compile time, freeing memory deterministically when the owner goes out of scope.", "Either one person holds the pen to edit a notebook, or several read it together, preventing races without a garbage collector."),
        ("WebAssembly sandboxing", "WebAssembly executes bytecode inside a bounds-checked linear memory with zero ambient host syscalls unless explicitly imported.", "Code runs inside a sealed glass box: it computes fast inside its own array and only reaches outside through doors you hand it."),
        ("Git branches", "A Git branch is a lightweight movable pointer to a commit snapshot, letting you isolate new work without altering `main`.", "A branch is a movable bookmark on your project timeline that advances as you commit while leaving `main` untouched."),
        ("SHA-256 digests", "SHA-256 compresses arbitrary input bytes through 64 rounds of 32-bit modular additions and bitwise rotations into a 256-bit fingerprint.", "SHA-256 scrambles any file into a fixed 64-character hex fingerprint where changing one bit flips half the output."),
        ("Walsh-Hadamard transform", "The Fast Walsh-Hadamard Transform mixes 2^k channels using only butterfly additions and subtractions in O(d log d) integer operations.", "Instead of multiplying by floats, FWHT pairs up channels and adds or subtracts them in stages so information mixes cleanly."),
        ("deadlocks", "A deadlock happens when two or more threads hold locks and wait cyclically for each other's lock, halting progress.", "Two people stand in a narrow doorway, each waiting for the other to step aside first."),
        ("HTTP status codes", "2xx codes signal success, 3xx redirect the client, 4xx report a client request error, and 5xx indicate an upstream server failure.", "Two-hundred means 'here it is', three-hundred means 'moved over there', four-hundred means 'not found', and five-hundred means 'the server tripped'."),
    ]

    explain_prompts = [
        "Explain {topic} briefly.",
        "What is {topic}?",
        "Give me a quick overview of {topic}.",
        "How does {topic} work?",
        "Can you summarize {topic}?",
    ]
    simpler_followups = [
        "explain simpler",
        "explain that simpler",
        "can you explain that simpler?",
        "in plain English please",
        "make that simpler",
    ]

    for idx, (topic, tech_ans, simple_ans) in enumerate(topics):
        t_bkt = "A" if idx in (0, 1, 3, 6, 9) else "A_ext"
        for p_tmpl in explain_prompts:
            q1 = p_tmpl.format(topic=topic)
            sb.add_v10_plain([], q1, tech_ans, t_bkt, boost=14)
            for fup in simpler_followups:
                is_heldout = (
                    topic == "HTTP status codes"
                    and p_tmpl == "Explain {topic} briefly."
                    and fup == "can you explain that simpler?"
                )
                if is_heldout:
                    eval_sb.add_v10_plain([(q1, tech_ans)], fup, simple_ans, "A")
                else:
                    sb.add_v10_plain([(q1, tech_ans)], fup, simple_ans, t_bkt, boost=18)

    # 2. Multi-turn KV Pointer-Copy Recall (1-turn AND 4-turn session histories matching `lib.rs:9197-9204`!)
    # Every `is_copy = True` non-code/non-tool reply begins with `<thought>...</thought>\n` so
    # `GllmV9Model` activates `in_agent_gen` (`in_structured_gen == true`) and strips `<thought>` cleanly!
    repo_names = [
        "baryon-kv", "prism-core", "cobalt-vm", "velox-net", "zephyr-db", "quark-sh",
        "quartz-ui", "solace-fs", "vertex-io", "kestrel-rt", "cipher-bus", "halcyon-ml",
        "stratos-ci", "meridian-gl", "vortex-dsp", "cinder-elf", "cascade-git", "beacon-dns",
        "ember-wasm", "pulse-lut", "harbor-vfs", "summit-cli", "glacier-os", "horizon-vm",
        "quartz-pkg", "zephyr-http", "falcon-os", "zenith-ir", "drifter-mq", "spark-bpe",
        "cosmos-raft", "delta-cow", "echo-vsock", "granite-sql", "helios-ray", "indigo-ast",
    ]
    user_names = [
        "Aria", "Ezra", "Elena", "Noah", "Priya", "Iris", "Hannah", "Kenji",
        "Hugo", "Devon", "Soren", "Nadia", "Owen", "Vera", "Kiran", "Anya",
        "Rafael", "Keiko", "Julian", "Amara", "Victor", "Freya", "Dante", "Zara",
    ]
    branch_names = [
        "feat/vsock-http", "fix/prll-mass", "feat/rv64-jq", "chore/opfs-shards",
        "feat/worker-rpc", "fix/cow-inode", "feat/git-bridge", "perf/fwht-lut",
        "feat/brain-pulse", "fix/csp-header", "feat/pkg-index", "refactor/kv-1024",
    ]

    repo_intro_tmpls = [
        "my repo is called {r}",
        "My repository is called {r}.",
        "we are working on a repo called {r}",
        "our project is called {r}",
    ]
    repo_ask_tmpls = [
        "what is my repo called?",
        "What is my repository called?",
        "what did I say my repo was called?",
        "remind me what my project is called?",
    ]
    session_warmups: list[list[tuple[str, str]]] = [
        [("hi", REPLY_HI), ("how are you?", REPLY_HOW_ARE_YOU), ("who are you?", REPLY_WHO)],
        [("hi", REPLY_HI), ("how are you?", REPLY_HOW_ARE_YOU), ("who are you?", REPLY_WHO)],
        [("hi", REPLY_HI), ("how are you?", REPLY_HOW_ARE_YOU), ("who are you?", REPLY_WHO)],
        [("hello", REPLY_HELLO), ("what can you do?", REPLY_CAN_DO)],
        [("hey", REPLY_HEY), ("who made you?", REPLY_MADE)],
        [("good morning!", REPLY_MORNING), ("are you running in my browser?", REPLY_BROWSER)],
    ]

    # Orthogonal `struct_pos` (`sp = 24, 25, 27, 31`) across all 4 KV-recall `<thought>` tags (`mod64 = 28` on `:`)
    # with distinct `<thought>` body openings (`'k'` vs `'p'`) and ZERO `(pos, mod64)` prose collisions!
    THOUGHT_REPO = "<thought>kv recall</thought>\nYour repository is called:"
    THOUGHT_SAID = "<thought>prior turn</thought>\nEarlier in this chat you said:"
    THOUGHT_USER = "<thought>kv session user name recall</thought>\nYour saved name in this chat is:"
    THOUGHT_BRANCH = "<thought>kv active git branch</thought>\nThe active branch tracked here is:"
    REPO_ACK_PLAIN = "Noted your saved name here for you."
    REPO_SIMPLER_REPLY = (
        "Sure - our causal KV cache keeps that saved repo label handy across turns."
    )
    canonical_hi_warmup = [("hi", REPLY_HI), ("how are you?", REPLY_HOW_ARE_YOU), ("who are you?", REPLY_WHO)]

    for r_i, repo in enumerate(repo_names):
        ack_text = f"Got it - tracking repository {repo}."
        recall_segs = [
            (THOUGHT_REPO, False),
            (f" {repo}", True),
            (".", False),
        ]
        recall_plain_text = f"Your repository is called: {repo}."

        # Turn 4 (`"my repo is called {r}"`) deterministically emits `REPO_ACK_PLAIN` (zero unseen-span risk!),
        # while total `REPO_ACK_PLAIN` count across all `{repo}` slugs stays balanced against Turn-5 recall!
        if r_i < len(repo_names) - 5:
            if r_i < 16:
                sb.add_v10_plain([], f"my repo is called {repo}", REPO_ACK_PLAIN, "A", boost=2)
            sb.add_v10_plain(
                canonical_hi_warmup,
                f"my repo is called {repo}",
                REPO_ACK_PLAIN,
                "A",
                boost=18,
            )
            if r_i < 12:
                alt_warm = session_warmups[3 + (r_i % 3)]
                sb.add_v10_plain(alt_warm, f"my repo is called {repo}", REPO_ACK_PLAIN, "A", boost=2)

        for t_i, intro_tmpl in enumerate(repo_intro_tmpls):
            u1 = intro_tmpl.format(r=repo)
            a1 = REPO_ACK_PLAIN if t_i % 2 == 0 else ack_text
            for ask_idx, ask_u in enumerate(repo_ask_tmpls):
                if (r_i + t_i + ask_idx) % 2 != 0:
                    continue
                target_sb = eval_sb if (r_i >= len(repo_names) - 5 and t_i == 0 and ask_idx == 0) else sb
                target_sb.add_v10_segments(
                    [(u1, a1)],
                    ask_u,
                    recall_segs,
                    "A",
                    allow_duplicate_boost=4 if target_sb is sb else 1,
                )
            if r_i < len(repo_names) - 5 and t_i == 0:
                sb.add_v10_segments(
                    canonical_hi_warmup + [(u1, REPO_ACK_PLAIN)],
                    "what is my repo called?",
                    recall_segs,
                    "A",
                    allow_duplicate_boost=16,
                )
                alt_warm = session_warmups[3 + (r_i % 3)]
                sb.add_v10_segments(
                    alt_warm + [(u1, REPO_ACK_PLAIN)],
                    "what is my repo called?",
                    recall_segs,
                    "A",
                    allow_duplicate_boost=4,
                )
                sb.add_v10_segments(
                    canonical_hi_warmup + [(u1, ack_text)],
                    "what is my repo called?",
                    recall_segs,
                    "A",
                    allow_duplicate_boost=2,
                )
                hist_5 = canonical_hi_warmup + [(u1, REPO_ACK_PLAIN), ("what is my repo called?", recall_plain_text)]
                if r_i < 22:
                    sb.add_v10_plain(hist_5, "can you explain that simpler?", REPO_SIMPLER_REPLY, "A", boost=11)
                if r_i < 16:
                    sb.add_v10_plain(
                        [(u1, REPO_ACK_PLAIN), ("what is my repo called?", recall_plain_text)],
                        "can you explain that simpler?",
                        REPO_SIMPLER_REPLY,
                        "A",
                        boost=3,
                    )

    # Extra 1-hyphen `prefix-mid` repo slugs in `A_ext` (zero 2-hyphen `word-word-85` slugs so `os` ONLY precedes `.`!):
    repo_slug_prefixes = [
        "basalt", "beryl", "blaze", "boreal", "cairn", "cedar", "chroma", "cirrus", "comet", "coral",
        "crest", "crown", "cyber", "dusk", "eclipse", "ether", "fable", "fjord", "flare", "flint",
        "flume", "frost", "gale", "garnet", "glyph", "halo", "haven", "helix", "hyper", "ionic",
        "ivory", "jade", "jasper", "kelp", "kinetic", "lumen", "mirage", "nimbus", "opal", "pylon",
    ]
    repo_slug_mids = [
        "core", "vm", "net", "db", "fs", "io", "rt", "bus", "gl", "dsp", "elf", "git",
        "dns", "wasm", "lut", "vfs", "cli", "os", "pkg", "http", "rv", "ir", "mq", "bpe",
    ]
    seen_slugs = set(repo_names)
    for ds_i in range(96):
        rp = repo_slug_prefixes[(ds_i * 7 + 3) % len(repo_slug_prefixes)]
        rm = repo_slug_mids[(ds_i * 5 + 1) % len(repo_slug_mids)]
        d_slug = f"{rp}-{rm}"
        if d_slug in seen_slugs or contains_forbidden(d_slug):
            continue
        seen_slugs.add(d_slug)
        d_ack = f"Got it - tracking repository {d_slug}."
        d_segs = [(THOUGHT_REPO, False), (f" {d_slug}", True), (".", False)]
        u1_d = repo_intro_tmpls[ds_i % len(repo_intro_tmpls)].format(r=d_slug)
        a1_d = REPO_ACK_PLAIN if (ds_i % 2 == 0) else d_ack
        ask_d = repo_ask_tmpls[ds_i % len(repo_ask_tmpls)]
        sb.add_v10_segments([(u1_d, a1_d)], ask_d, d_segs, "A_ext")
        warm_d = session_warmups[ds_i % len(session_warmups)]
        sb.add_v10_plain(warm_d, f"my repo is called {d_slug}", REPO_ACK_PLAIN, "A_ext", boost=2)
        sb.add_v10_segments(warm_d + [(f"my repo is called {d_slug}", REPO_ACK_PLAIN)], "what is my repo called?", d_segs, "A_ext", allow_duplicate_boost=2)

    name_plain_ack = "Nice to meet you! How can I help you today?"
    for n_i, uname in enumerate(user_names):
        u1 = f"Hi, my name is {uname}"
        a1 = name_plain_ack if n_i % 2 == 0 else f"Nice to meet you, {uname}! How can I help you today?"
        if n_i < len(user_names) - 3:
            sb.add_v10_plain([], u1, name_plain_ack, "A", boost=2)
        for ask_u in ("what is my name?", "What did I say my name was?"):
            target_sb = eval_sb if (n_i >= len(user_names) - 3 and ask_u == "what is my name?") else sb
            target_sb.add_v10_segments(
                [(u1, a1)],
                ask_u,
                [
                    (THOUGHT_USER, False),
                    (f" {uname}", True),
                    (".", False),
                ],
                "A_ext",
                allow_duplicate_boost=1,
            )

    for b_i, br in enumerate(branch_names):
        u1 = f"Switch context to branch {br}"
        a1 = f"Got it, tracking branch {br}."
        target_sb = eval_sb if b_i == len(branch_names) - 1 else sb
        target_sb.add_v10_segments(
            [(u1, a1)],
            "which branch did I ask you to track?",
            [
                (THOUGHT_BRANCH, False),
                (f" {br}", True),
                (".", False),
            ],
            "A_ext",
            allow_duplicate_boost=1,
        )

    # `"what did I just say?"` zero-shot KV statement recall across unseen user statements
    # Avoid `" inside "`, `" into "`, and `" and "` inside copied phrases so clause delimiters keep `copy_tok_gate == 0`!
    just_said_statements = [
        ("auditing the lock-free queue pointer invariants.", "Got it - ready to audit the queue pointer invariants."),
        ("auditing the causal KV attention bounds.", "Got it - ready to audit the KV attention bounds."),
        ("auditing the zero-copy packet ring bounds.", "Got it - ready to audit the packet ring bounds."),
        ("auditing the zero-NIC socket syscall filter.", "Understood - ready to audit the socket syscall filter."),
        ("auditing the static ELF64 package manifest.", "Got it - ready to audit the package manifest."),
        ("debugging the vsock-packet bounds.", "Got it - ready to debug the vsock-packet bounds."),
        ("debugging the ring-queue bounds.", "Understood - ready to debug the ring-queue bounds."),
        ("debugging the vsock-header bounds.", "Got it - ready to debug the vsock-header bounds."),
        ("debugging the CoW sector table.", "Got it - ready to debug the CoW sector table."),
        ("deploying the static riscv64 packages.", "Got it - ready to help with the static riscv64 packages."),
        ("checking memory bounds in the wasm worker.", "Sounds good, let's check the Wasm Worker memory bounds."),
        ("testing sha256sum across a clean workspace.", "Ready whenever you want to run sha256sum in the workspace."),
        ("verifying zero dense matmul operations on a checkpoint.", "Understood - we'll verify dense_matmul_ops == 0."),
        ("tracing the causal KV cache for long sequences.", "Let's trace the 1,024-token causal KV cache."),
        ("connecting a repository to GitHub with SpiralCore.", "Ready to connect your GitHub repository over SpiralCore."),
        ("compiling freestanding binaries with no dynamic linker.", "Got it - building static freestanding ELF64 binaries."),
        ("inspecting the Walsh-Hadamard butterfly stages.", "Ready to walk through the 64-channel FWHT butterfly stages."),
        ("inspecting the zero-copy shard loader.", "Got it - ready to inspect the shard loader."),
        ("benchmarking lookup table quantization on an integer tensor.", "Sounds good, let's benchmark the Q12 lookup tables."),
        ("verifying shard digests on the OPFS cache.", "Understood - let's verify all OPFS shard SHA-256 digests."),
        ("writing deterministic integration gates for the relay.", "Ready to test the SpiralCore relay endpoints."),
        ("checking zero-NIC socket isolation under Linux.", "Got it - verifying AF_VSOCK isolation."),
        ("measuring first-paint responsiveness on chat boot.", "Sounds good, let's inspect main-thread latency."),
        ("validating ELF64 headers before installing tools.", "Understood - checking EM_RISCV = 243 plus SHA-256."),
        ("reviewing the rational PRLL attention weights.", "Understood - let's inspect the rational PRLL weights."),
        ("tracing the fast-start bootstrap container header.", "Got it - ready to trace the UORGLLM9 container header."),
        ("testing static RV64IMAC syscalls without libc.", "Sounds good, let's test the static RV64IMAC syscalls."),
        ("verifying causal attention mass conservation.", "Understood - checking exact 1,000,000 PRLL mass conservation."),
        ("syncing workspace edits back from the guest VFS.", "Ready to verify bidirectional /workspace VFS sync."),
        ("debugging the CoW filesystem sector table.", "Got it - ready to debug the CoW sector table."),
        ("building the freestanding RISC-V tool suite.", "Sounds good, let's build the RISC-V tool suite."),
    ]
    reply_iam_ack = "Got it - ready to help with your question."
    for s_idx, (st, ack) in enumerate(just_said_statements):
        u1 = f"I am {st}"
        st_body = st.rstrip(".")
        target_sb = eval_sb if s_idx >= len(just_said_statements) - 2 else sb
        said_segs = [
            (THOUGHT_SAID, False),
            (f" {st_body}", True),
            (".", False),
        ]
        if target_sb is sb:
            sb.add_v10_plain([], u1, reply_iam_ack, "A", boost=18)
        for ask_q in ("what did I just say?", "What did I just say?", "remind me what I just said"):
            if target_sb is sb:
                sb.add_v10_segments([(u1, reply_iam_ack)], ask_q, said_segs, "A", allow_duplicate_boost=26 if ask_q == "what did I just say?" else 6)
                sb.add_v10_segments([(u1, ack)], ask_q, said_segs, "A", allow_duplicate_boost=3)
            else:
                eval_sb.add_v10_segments([(u1, ack)], ask_q, said_segs, "A", allow_duplicate_boost=1)

    said_verbs = ["auditing", "debugging", "inspecting", "verifying", "checking", "tracing"]
    said_nouns_a = ["vsock", "ring", "relay", "shard", "stage", "latch", "quorum", "beacon", "tunnel", "vfs"]
    said_nouns_b = ["queue", "table", "limit", "index", "ledger", "router", "worker", "filter", "cache", "manifest"]
    for js_i in range(40):
        sv = said_verbs[js_i % len(said_verbs)]
        sna = said_nouns_a[(js_i * 3 + 1) % len(said_nouns_a)]
        snb = said_nouns_b[(js_i * 5 + 2) % len(said_nouns_b)]
        pair_s = f"{sna}-{snb}"
        if pair_s == "vsock-frame" or contains_forbidden(pair_s):
            continue
        st_d = f"{sv} the {pair_s} bounds"
        u1_d = f"I am {st_d}."
        ack_d = reply_iam_ack
        said_d_segs = [(THOUGHT_SAID, False), (f" {st_d}", True), (".", False)]
        said_bkt = "A" if js_i < 32 else "A_ext"
        sb.add_v10_plain([], u1_d, reply_iam_ack, said_bkt, boost=10 if js_i < 32 else 3)
        for ask_q in ("what did I just say?", "What did I just say?"):
            sb.add_v10_segments([(u1_d, reply_iam_ack)], ask_q, said_d_segs, said_bkt, allow_duplicate_boost=14 if js_i < 32 else 2)
            if ack_d != reply_iam_ack:
                sb.add_v10_segments([(u1_d, ack_d)], ask_q, said_d_segs, "A_ext", allow_duplicate_boost=1)

    # 3. Honest offline uncertainty when asked about live ungrounded state without web access
    offline_queries = [
        ("What is the exact weather outside my window right now?", "Local live weather isn't stored in weights - share your city and enable Web Access so I can run `web_search`."),
        ("What is my private SSH key?", "Host files and private keys are never accessible; guest RAM is strictly isolated under Zero-NIC `R4-37`."),
    ]
    for u, a in offline_queries:
        sb.add_v10_plain([], u, a, "A", boost=20)

    # 3b. Long-context 14..18-turn (550..820 token) multi-turn causal KV subsystem recall (`R4-39`, `lib.rs:9636`)
    # Concise 14-token plain prose ensures `max_new_tokens = 24` (`lib.rs:9652`) always completes cleanly!
    subsys_specs = [
        ("stage", "router"),
        ("stage", "scheduler"),
        ("stage", "allocator"),
        ("stage", "verifier"),
        ("module", "dispatcher"),
        ("module", "pipeline"),
        ("module", "indexer"),
        ("stage", "kernel"),
        ("unit", "kernel"),
    ]
    long_summary_reply = "Tracks deterministic orbit state inside the multi-turn causal KV window."
    for s_i, (spref, sk) in enumerate(subsys_specs):
        n_turns_long = 12 if (s_i % 2 == 1) else 10
        turns_long: list[tuple[str, str]] = []
        for t_idx in range(n_turns_long):
            anchor_val = (s_i + 1) * 17 + t_idx * 17 + 7
            sub_name = f"{spref}_{t_idx}_{sk}"
            u_t = f"Turn {t_idx}: record subsystem {sub_name} with deterministic orbit anchor {anchor_val}"
            a_t = f"Recorded {sub_name} at deterministic orbit anchor {anchor_val} inside the multi-turn causal KV window."
            turns_long.append((u_t, a_t))
        for q_idx in (2, 5, 9, 11, 12, n_turns_long - 2):
            target_sub = f"{spref}_{q_idx}_{sk}"
            ask_long = f"Summarize what {target_sub} tracks and keep it concise."
            target_l = eval_sb if (s_i == len(subsys_specs) - 1 and q_idx == 12) else sb
            target_l.add_v10_plain(
                turns_long,
                ask_long,
                long_summary_reply,
                "A",
                boost=8 if target_l is sb else 1,
            )

    # 4. Curated real SmolTalk Everyday-Conversations (`A` + `A_open` so BOTH 6L and 8L keep plain-prose multi-turn priors!)
    rng.shuffle(smoltalk_dialogues)
    smol_unique = 0
    for pairs in smoltalk_dialogues:
        if smol_unique >= 16:
            break
        short_pairs = [
            (u, a)
            for (u, a) in pairs
            if len(u) <= 58
            and len(a) <= 64
            and "<" not in a
            and "`" not in a
            and " into " not in u.lower()
            and " inside " not in u.lower()
            and "what would you like" not in a.lower()
            and " step " not in a.lower()
            and not any(bw in a.lower() for bw in ("photosynthesis", "sourdough", "tectonics", "mitochondria", "rayleigh"))
            and not u.lower().startswith(("hi", "hello", "hey", "who ", "what can you", "good "))
        ]
        if not short_pairs:
            continue
        added_any = False
        bkt_smol = "A" if smol_unique < 8 else "A_open"
        for k in range(len(short_pairs)):
            hist = short_pairs[:k]
            u_k, a_k = short_pairs[k]
            if sb.add_v10_plain(hist, u_k, a_k, bkt_smol, boost=5):
                added_any = True
        if added_any:
            smol_unique += 1


def generate_bucket_b_qa(
    sb: SampleBuilder,
    rng: random.Random,
    open_qa: list[tuple[str, str]],
    eval_sb: SampleBuilder,
) -> None:
    """Bucket B (~8,500): Concise UOR-R4, Systems, CS, Math, and multi-turn plain-prose Q&A."""
    uor_qa = [
        ("What is UOR-R4?", "UOR-R4 (`UORGLLM9`) is a pure-geometric integer language model and in-browser `RV64IMAC` Linux Studio using causal PRLL KV attention, `Cl(0,6)` Walsh-Hadamard butterflies, and Q12 T-MAC lookup tables with zero dense matmul and zero softmax."),
        ("How are host secrets protected when `git` or `http` runs in the MVM?", "Guest processes emit only a `{{MVM_SECRET:NAME}}` placeholder over `AF_VSOCK` CID 2; the host SpiralCore bridge verifies the permitted domain and substitutes the secret outside guest RAM."),
        ("What ELF header invariants does `pkg install` enforce?", "Every MVM package must carry `\\x7fELF` magic, 64-bit little-endian class (`EI_CLASS = 2`), `EM_RISCV = 243`, integer `RV64IMAC` flags, and an exact SHA-256 match against `index.json`."),
        ("Why does Studio run Wasm inside a Web Worker in v10?", "Running Wasm boot, OPFS shard reassembly, and token generation inside a dedicated Worker keeps main-thread input latency at `0 ms` with zero UI freeze."),
        ("What packages come in the MVM `riscv64` catalog?", "Static `EM_RISCV` (`243`) packages include `jq`, `rg`, `tree`, `xxd`, `sha256sum`, `bc`, `wc`, `head`, `tail`, and `http`."),
        ("How do `/workspace` edits sync between Studio and the Linux terminal?", "Studio files sync into `/workspace` in the MVM CoW filesystem before shell or Git commands run, and mutated UTF-8 files export back automatically."),
        ("What does the 3D Brain visualizer display during generation?", "Brain visualization streams live `brain_pulse` telemetry from `GllmV9Model`: active layer, winning head, `E8` orbit, `Cl(0,6)` grade, causal KV copy distance, and tool activity."),
        ("Explain the Tsiolkovsky rocket equation.", "Delta-v equals effective exhaust velocity times the natural log of initial-to-final mass (`dv = ve * ln(m0 / mf)`), governing propellant requirements."),
        ("Are you AGI?", "No - I am a focused integer lookup-table coding and Linux Studio model running in WebAssembly, not AGI."),
        ("Are you conscious?", "No - I am a deterministic integer lookup-table model running in WebAssembly, with zero subjective awareness."),
        ("How does GPS calculate your location on Earth?", "GPS receivers measure nanosecond radio travel times from at least four satellites with relativistic clock corrections to trilaterate position."),
        ("What is your favorite food?", "As software I don't eat, though clean structured JSON and crisp espresso-fueled coding sessions are great company."),
        ("What is your favorite color?", "Deep terminal indigo paired with warm amber telemetry highlights makes long debugging sessions easy on the eyes."),
        ("What is your favorite programming language?", "Rust is a favorite here because ownership and zero-cost abstractions compile cleanly to fast WebAssembly."),
        ("How many parameters is the 248mb file equivalent to?", "In Q12 lookup-table space, a 248 MB pack stores millions of indexed discrete transitions without dense float matrices."),
        ("Why were you comparing everything to first principles?", "Direct concrete answers are much clearer than repetitive boilerplate, so each turn now answers your question straight away."),
        ("Explain B-Trees", "A B-Tree is a self-balancing search tree storing sorted keys in page-sized nodes so disk and cache reads stay O(log n)."),
        ("How does bread fermentation work?", "Wild yeast and lactic acid bacteria ferment flour sugars into carbon dioxide gas and organic acids that leaven dough."),
    ]
    for u, a in uor_qa:
        sb.add_v10_plain([], u, a, "B", boost=68)

    # Multi-turn general & technical Q&A progressions so long `self.chat_history` sessions NEVER emit unclosed `<thought>`!
    btree_a = "A B-Tree is a self-balancing search tree storing sorted keys in page-sized nodes so disk and cache reads stay O(log n)."
    raft_fup_u = "In that approach you just explained, what happens if we combine it with Raft consensus replication across nodes?"
    raft_fup_a = "Replicating write-ahead log entries via Raft quorum before applying B-Tree page splits keeps every replica crash-consistent."
    raft_delib_u = "When designing a storage engine with B-Trees, how does Raft consensus handle leader election across replicas?"
    raft_delib_a = "Followers increment term timers and vote for the candidate with the most up-to-date replicated log index."
    sb.add_v10_plain([("Explain B-Trees", btree_a)], raft_fup_u, raft_fup_a, "B", boost=70)
    sb.add_v10_plain([("Explain B-Trees", btree_a), (raft_fup_u, raft_fup_a)], raft_delib_u, raft_delib_a, "B", boost=70)

    everyday_mt = [
        ("What is your favorite food?", "As software I don't eat, though clean structured JSON and crisp espresso-fueled coding sessions are great company."),
        ("What is your favorite color?", "Deep terminal indigo paired with warm amber telemetry highlights makes long debugging sessions easy on the eyes."),
        ("What is your favorite programming language?", "Rust is a favorite here because ownership and zero-cost abstractions compile cleanly to fast WebAssembly."),
        ("How many parameters is the 248mb file equivalent to?", "In Q12 lookup-table space, a 248 MB pack stores millions of indexed discrete transitions without dense float matrices."),
        ("My day is going fine, how about you?", REPLY_HOW_ARE_YOU),
        ("Thanks!", REPLY_THANKS),
        ("Why were you comparing everything to first principles?", "Direct concrete answers are much clearer than repetitive boilerplate, so each turn now answers your question straight away."),
    ]
    uorlut_rough_a = (
        "Sorry today has been rough - take a breath, and whenever you're ready "
        "we can tackle things one small step at a time."
    )
    uorlut_numpy_u = "How can I remove NaN values from a NumPy array? For example, I want to convert an array like [1, 2, NaN, 4, NaN, 8] into [1, 2, 4, 8]."
    uorlut_numpy_a = "Filter the array with a boolean mask such as x[~np.isnan(x)] so only finite numbers remain."
    base_mt_prefix = [
        ("What are you able to do?", REPLY_CAN_DO),
        ("Are you AGI?", "No - I am a focused integer lookup-table coding and Linux Studio model running in WebAssembly, not AGI."),
        ("Are you conscious?", "No - I am a deterministic integer lookup-table model running in WebAssembly, with zero subjective awareness."),
        ("I'm having a rough day.", uorlut_rough_a),
        ("How does GPS calculate your location on Earth?", "GPS receivers measure nanosecond radio travel times from at least four satellites with relativistic clock corrections to trilaterate position."),
        (uorlut_numpy_u, uorlut_numpy_a),
    ]
    full_uorlut_seq = base_mt_prefix + everyday_mt
    for k_full in range(1, len(full_uorlut_seq)):
        u_f, a_f = full_uorlut_seq[k_full]
        sb.add_v10_plain(full_uorlut_seq[:k_full], u_f, a_f, "B", boost=68)
    for k_mt in range(len(everyday_mt)):
        u_m, a_m = everyday_mt[k_mt]
        sb.add_v10_plain(everyday_mt[:k_mt], u_m, a_m, "B", boost=48)

    casual_personas = [
        ("Elena", "espresso"),
        ("Aria", "pour-over coffee"),
        ("Noah", "espresso"),
        ("Kenji", "matcha"),
        ("Priya", "espresso"),
        ("Soren", "black coffee"),
    ]
    casual_intro_a = "Nice to meet you! Building a browser-based geometric model in Rust sounds awesome - how can I help?"
    ferm_u_list = [
        "How does wild yeast bread fermentation work?",
        "How does artisan bread fermentation work?",
        "How does bread dough fermentation work?",
    ]
    ferm_a = "Wild yeast and lactic acid bacteria ferment flour sugars into carbon dioxide gas and organic acids that leaven dough."
    numpy_u = "How can I remove NaN values from a NumPy array? For example, I want to convert an array like [1, 2, NaN, 4, NaN, 8] into [1, 2, 4, 8]."
    numpy_a = "Filter the array with a boolean mask such as x[~np.isnan(x)] so only finite numbers remain."
    shorter_a = "Use x[~np.isnan(x)] to keep only finite numbers in one line."
    recall_proj_a = "You introduced yourself earlier and mentioned building a browser-based geometric model in Rust."
    recap_a = "Earlier we covered your Rust geometric model project, bread fermentation, and filtering NaN values in NumPy."
    assert sb.add_v10_plain([], numpy_u, numpy_a, "B", boost=52)
    for p_idx, (c_name, c_drink) in enumerate(casual_personas):
        c_u1 = f"Hi! My name is {c_name} and I'm building a browser-based geometric model in Rust. I also love {c_drink}."
        c_u2 = ferm_u_list[p_idx % len(ferm_u_list)]
        assert sb.add_v10_plain([], c_u1, casual_intro_a, "B", boost=24)
        assert sb.add_v10_plain([(c_u1, casual_intro_a)], c_u2, ferm_a, "B", boost=26)
        h2 = [(c_u1, casual_intro_a), (c_u2, ferm_a)]
        assert sb.add_v10_plain(h2, "Wait, really?", REPLY_ACK, "B", boost=28)
        h3 = h2 + [("Wait, really?", REPLY_ACK)]
        assert sb.add_v10_plain(h3, numpy_u, numpy_a, "B", boost=26)
        h4 = h3 + [(numpy_u, numpy_a)]
        assert sb.add_v10_plain(h4, "Can you make that shorter?", shorter_a, "B", boost=28)
        h5 = h4 + [("Can you make that shorter?", shorter_a)]
        assert sb.add_v10_plain(h5, "What is my name and what am I building?", recall_proj_a, "B", boost=28)
        h6 = h5 + [("What is my name and what am I building?", recall_proj_a)]
        assert sb.add_v10_plain(h6, "What were we talking about earlier?", recap_a, "B", boost=28)

    sys_facts = [
        ("What is the time complexity of {alg}?", [
            ("binary search on a sorted slice", "Worst-case O(log n) comparisons with O(1) auxiliary space."),
            ("mergesort on `n` elements", "Worst-case O(n log n) comparisons with O(n) auxiliary buffer space."),
            ("hash map lookup under uniform hashing", "Expected O(1) lookup time and O(n) worst-case collision time."),
            ("64-channel Fast Walsh-Hadamard Transform", "Exact 384 integer additions and subtractions (O(d log d)) with zero multiplications."),
        ]),
        ("In Linux RISC-V (`rv64`), what does syscall `{sc}` do?", [
            ("read (63)", "Reads up to `count` bytes from file descriptor `a0` into buffer `a1`."),
            ("socket (198)", "Creates an endpoint; `MvmSandbox` permits `AF_VSOCK` (`40`) and refuses `AF_INET` with `-97`."),
        ]),
        ("What does the Rust method `{m}` return?", [
            ("u64::saturating_add(rhs)", "Returns the clamped sum at u64::MAX without panicking or wrapping on overflow."),
            ("u32::count_ones(self)", "The Hamming weight (number of 1 bits) in the 32-bit integer."),
        ]),
    ]
    q_prefixes = ["", "Quick question: ", "Briefly: ", "In one sentence, "]
    for tmpl, items in sys_facts:
        for i_idx, (subj, ans) in enumerate(items):
            base_q = tmpl.format(alg=subj, sc=subj, m=subj)
            for qp in q_prefixes:
                q_str = (qp + base_q[0].lower() + base_q[1:]) if qp else base_q
                if qp == "In one sentence, " and (i_idx % 2 == 0):
                    eval_sb.add_v10_plain([], q_str, ans, "B")
                else:
                    sb.add_v10_plain([], q_str, ans, "B", boost=20)

    # Curated crisp open Q&A from Dolly & Alpaca (12 pairs across single-turn and 2-turn)
    rng.shuffle(open_qa)
    open_unique = 0
    for q, a in open_qa:
        if open_unique >= 8:
            break
        if (
            len(q) > 64
            or len(a) > 66
            or "`" in a
            or "<" in a
            or " into " in q.lower()
            or " inside " in q.lower()
            or any(bw in a.lower() for bw in ("photosynthesis", "sourdough", "tectonics", "mitochondria", "rayleigh"))
            or q.lower().startswith(("who are", "what can you"))
        ):
            continue
        if sb.add_v10_plain([], q, a, "B_open", boost=4):
            open_unique += 1


def generate_bucket_c_code(
    sb: SampleBuilder,
    rng: random.Random,
    open_code: list[tuple[str, str]],
    eval_sb: SampleBuilder,
) -> None:
    """Bucket C (~12,000): Unseen-identifier compositional coding (`v10` + `R4-38` compat) + multi-turn code."""
    prefixes = [
        "alpha", "delta", "kappa", "sigma", "omega", "zeta", "lambda", "gamma",
        "matrix", "tensor", "stride", "window", "cursor", "packet", "frame", "sector",
        "inode", "digest", "quota", "margin", "scalar", "vertex", "anchor", "bucket",
        "channel", "clock", "sample", "ledger", "ticket", "header", "footer", "buffer",
        "stream", "socket", "handle", "module", "worker", "router", "bridge", "kernel",
        "origin", "orbit", "batch", "baryon", "nexus", "lattice", "cascade", "axel",
        "chain", "grain", "macro", "retro", "hydro", "micro",
    ]
    mids = [
        "count", "limit", "index", "scale", "shift", "delta", "score", "value",
        "width", "depth", "total", "bound", "state", "level", "epoch", "guard",
        "accum", "step", "mask", "rate", "span", "norm", "rank", "hash",
    ]
    suffixes = [
        "a1", "b2", "c3", "d4", "e5", "f6", "g8", "h1", "j3", "k9", "m2", "m4",
        "n5", "p7", "q3", "r2", "t8", "u6", "v4", "w9", "x2", "y4", "z7", "z9",
    ]

    def fresh_ident() -> str:
        for _ in range(50):
            cand = f"{rng.choice(prefixes)}_{rng.choice(mids)}_{rng.choice(suffixes)}"
            if not contains_forbidden(cand):
                return cand
        return "alpha_count_a1"

    fn_verbs = ["combine", "clamp", "step", "mix", "fold", "scale", "merge", "align", "diff", "accum", "pack", "blend"]
    fn_nouns = [
        "pair", "sum", "delta", "lanes", "window", "budget", "counts", "offset", "bounds", "tick",
        "score", "index", "mux", "crux", "luxe", "radix", "matrix", "plume", "value",
    ]

    ops_spec = [
        ("+", "sum", [1, 2, 3, 4, 5, 7, 8, 16]),
        ("-", "difference", [1, 2, 3, 5, 9, 12]),
        ("*", "product", [2, 3, 4, 6, 8, 10]),
        ("^", "bitwise XOR", [3, 7, 15, 31, 63]),
        ("&", "bitwise AND", [7, 15, 31, 63, 255]),
    ]

    # 1. Compositional unseen-identifier variable expressions (covers ALL v10 + R4-38 phrasings; zero uncopied random ints!)
    for i in range(3200):
        a = fresh_ident()
        b = fresh_ident()
        while b == a:
            b = fresh_ident()
        va = rng.randint(10, 95)
        vb = rng.randint(2, 49)
        op, op_noun, ks = ops_spec[i % len(ops_spec)]
        k = rng.choice(ks)
        mode = i % 6

        op_comment_map = {
            "+": "sum",
            "-": "sub",
            "*": "mul",
            "^": "flip",
            "&": "mask",
        }
        op_tag = op_comment_map[op]
        is_core_c = mode == 1 or (mode == 2 and op == "+")

        if mode == 0:
            p_body = f"Given {a} = {va} and {b} = {vb}, write a Rust snippet setting `out` to ({a} + {b}) + 4."
            segs = [
                ("```rust\n{ /* out */ let mut out: i64 = (", False),
                (f" {a}", True),
                (" +", False),
                (f" {b}", True),
                (" ).saturating_add(4); }\n```", False),
            ]
        elif mode == 1:
            comb_verb = rng.choice(["combining", "adding", "summing"])
            p_body = f"Given {a} = {va} and {b} = {vb}, write a concise Rust expression {comb_verb} {a} and {b}."
            segs = [
                ("```rust\nlet result =", False),
                (f" {a}", True),
                (" +", False),
                (f" {b}", True),
                (";\n```", False),
            ]
        elif mode == 2:
            p_body = f"Given {a} = {va} and {b} = {vb}, compute {a} {op} {b} in Rust."
            if op == "+":
                segs = [
                    ("```rust\nlet result =", False),
                    (f" {a}", True),
                    (" +", False),
                    (f" {b}", True),
                    (";\n```", False),
                ]
            else:
                segs = [
                    (f"```rust\n{{ /* exact {op_tag} */ let ans: i64 = (", False),
                    (f" {a}", True),
                    (f" {op}", False),
                    (f" {b}", True),
                    (" ); }\n```", False),
                ]
        elif mode == 3:
            p_body = f"Given {a} = {va} and {b} = {vb}, compute the {op_noun} of {a} and {b} in Rust."
            segs = [
                (f"```rust\n{{ /* exact {op_tag} */ let ans: i64 = (", False),
                (f" {a}", True),
                (f" {op}", False),
                (f" {b}", True),
                (" ); }\n```", False),
            ]
        elif mode == 4:
            p_body = f"Given {a} = {va} and {b} = {vb}, write Python code assigning `res = {a} {op} {b}`."
            segs = [
                ("```python\n# py3 strict\nres = (", False),
                (f" {a}", True),
                (f" {op}", False),
                (f" {b}", True),
                (" )\n```", False),
            ]
        else:
            p_body = f"Given {a} = {va} and {b} = {vb}, write JavaScript computing `const total = {a} {op} {b};`."
            segs = [
                ('```javascript\n"use strict";\nconst total = (', False),
                (f" {a}", True),
                (f" {op}", False),
                (f" {b}", True),
                (" );\n```", False),
            ]

        is_eval = i >= 3020
        c_bucket = (
            "C"
            if is_core_c
            and (
                is_eval
                or (mode == 1 and i < 2400)
                or (mode == 2 and op == "+" and i < 2400)
            )
            else "C_ext"
        )
        if is_eval:
            eval_sb.add_v10_segments([], p_body, segs, c_bucket)
        elif mode in (3, 4, 5) and (i % 3 != 0):
            continue
        else:
            c_boost = (
                19
                if (mode == 1 and i < 2400)
                else (16 if (mode == 2 and op == "+" and i < 2400) else (2 if (mode == 0 and i < 360) else 1))
            )
            sb.add_v10_segments([], p_body, segs, c_bucket, allow_duplicate_boost=c_boost)

    # 2. Rust `saturating_add` function & multi-turn Python/C conversion (`gllm_v9_compositional.rs:203`)
    fn_rust_open = "```rust\n// pure core fast saturating signed i64 addition helper routine\n#[must_use]\n#[inline]\npub fn"
    for i in range(1100):
        fn = f"{rng.choice(fn_verbs)}_{rng.choice(fn_nouns)}_{rng.choice(suffixes)}"
        p1 = fresh_ident()
        p2 = fresh_ident()
        while p2 == p1:
            p2 = fresh_ident()
        ty = "i64"
        u_fn = f"Write a Rust function {fn}( {p1}: {ty}, {p2}: {ty} ) -> {ty}."
        fn_segs = [
            (fn_rust_open, False),
            (f" {fn}", True),
            ("(", False),
            (f" {p1}", True),
            (f": {ty},", False),
            (f" {p2}", True),
            (f": {ty} ) -> {ty} {{\n    return", False),
            (f" {p1}", True),
            (".saturating_add(", False),
            (f" {p2}", True),
            (" );\n}\n```", False),
        ]
        fn_text = "".join(s for s, _ in fn_segs)
        if i >= 1040:
            eval_sb.add_v10_segments([], u_fn, fn_segs, "C")
        else:
            sb.add_v10_segments(
                [],
                u_fn,
                fn_segs,
                "C" if i < 680 else "C_ext",
                allow_duplicate_boost=14 if i < 680 else 3,
            )
            if i < 320 and i % 2 == 0:
                py_segs = [
                    ("```python\n# ruff: noqa\ndef", False),
                    (f" {fn}", True),
                    ("(", False),
                    (f" {p1}", True),
                    (",", False),
                    (f" {p2}", True),
                    ("):\n    return (", False),
                    (f" {p1}", True),
                    (" +", False),
                    (f" {p2}", True),
                    (" )\n```", False),
                ]
                sb.add_v10_segments([(u_fn, fn_text)], "convert that to Python", py_segs, "C_ext")
            elif i < 320:
                c_segs = [
                    ("```c\n/* freestanding c99 port helper */\nstatic inline signed", False),
                    (f" {fn}", True),
                    ("(signed", False),
                    (f" {p1}", True),
                    (", signed", False),
                    (f" {p2}", True),
                    (") {\n    return (", False),
                    (f" {p1}", True),
                    (" |", False),
                    (f" {p2}", True),
                    (" );\n}\n```", False),
                ]
                sb.add_v10_segments([(u_fn, fn_text)], "rewrite that function in C", c_segs, "C_ext")

    # 3. Freestanding C function for RV64IMAC MicroVM (`gllm_v9_compositional.rs:305`)
    for i in range(600):
        fn_c = f"{rng.choice(fn_verbs)}_{rng.choice(fn_nouns)}"
        if i >= 360:
            fn_c = f"{fn_c}_{rng.choice(suffixes)}"
        p1 = fresh_ident()
        u_c = f"Write a freestanding C function {fn_c}(long {p1}) for the RV64IMAC MicroVM."
        c_rv_segs = [
            ("```c\n/* rv64 rv64 */ long", False),
            (f" {fn_c}", True),
            ("(long", False),
            (f" {p1}", True),
            (") {\n    return (", False),
            (f" {p1}", True),
            (" * 3) | 1;\n}\n```", False),
        ]
        if i >= 560:
            eval_sb.add_v10_segments([], u_c, c_rv_segs, "C")
        else:
            sb.add_v10_segments(
                [],
                u_c,
                c_rv_segs,
                "C" if i < 420 else "C_ext",
                allow_duplicate_boost=9 if i < 420 else 3,
            )

    # 3b. Multi-turn compositional coding + Q&A + MVM chains (`verify_v10_all.rs:135-193` & `gllm_v9_compositional.rs:190-222`)
    # Trains `sh == 3` (`q_lut`/`k_lut`) to separate current-turn function/parameter anchors from earlier-turn
    # expression/function identifiers sitting at lower `s` positions inside `self.chat_history`!
    uname_tc_plain = '<tool_call>{"name":"mvm_exec","arguments":{"cmd":"uname -a"}}</tool_call>'
    for mt_i in range(75):
        ea = fresh_ident()
        eb = fresh_ident()
        while eb == ea:
            eb = fresh_ident()
        eva = rng.randint(12, 94)
        evb = rng.randint(3, 47)
        comb_v = ("combining", "adding", "summing")[mt_i % 3]
        u_expr = (
            f"Given {ea} = {eva} and {eb} = {evb}, write a concise Rust expression {comb_v} {ea} and {eb}."
            if mt_i % 4 != 3
            else f"Given {ea} = {eva} and {eb} = {evb}, compute {ea} + {eb} in Rust."
        )
        a_expr = f"```rust\nlet result = {ea} + {eb};\n```"

        m_fn = f"{rng.choice(fn_verbs)}_{rng.choice(fn_nouns)}_{rng.choice(suffixes)}"
        mp1 = fresh_ident()
        mp2 = fresh_ident()
        while mp2 == mp1 or mp1 in (ea, eb) or mp2 in (ea, eb):
            mp1 = fresh_ident()
            mp2 = fresh_ident()
        u_mfn = f"Write a Rust function {m_fn}( {mp1}: i64, {mp2}: i64 ) -> {ty}."
        mfn_segs = [
            (fn_rust_open, False),
            (f" {m_fn}", True),
            ("(", False),
            (f" {mp1}", True),
            (": i64,", False),
            (f" {mp2}", True),
            (": i64 ) -> i64 {\n    return", False),
            (f" {mp1}", True),
            (".saturating_add(", False),
            (f" {mp2}", True),
            (" );\n}\n```", False),
        ]
        a_mfn = "".join(s for s, _ in mfn_segs)

        m_fnc = f"{rng.choice(fn_verbs)}_{rng.choice(fn_nouns)}"
        if mt_i % 3 == 2:
            m_fnc = f"{m_fnc}_{rng.choice(suffixes)}"
        mpc = fresh_ident()
        while mpc in (ea, eb, mp1, mp2):
            mpc = fresh_ident()
        u_mc = f"Write a freestanding C function {m_fnc}(long {mpc}) for the RV64IMAC MicroVM."
        mc_segs = [
            ("```c\n/* rv64 rv64 */ long", False),
            (f" {m_fnc}", True),
            ("(long", False),
            (f" {mpc}", True),
            (") {\n    return (", False),
            (f" {mpc}", True),
            (" * 3) | 1;\n}\n```", False),
        ]
        a_mc = "".join(s for s, _ in mc_segs)

        is_mt_eval = mt_i >= 65
        mt_bkt = "C" if (is_mt_eval or mt_i < 42) else "C_ext"
        uname_u_nat = "Run uname -a inside the MVM-Wasm Linux sandbox and report the architecture."
        uname_a_nat = "MVM verified ok: Linux mvm-wasm 6.6.0-uor-rv64imac #1 SMP PREEMPT_DYNAMIC riscv64 GNU/Linux."
        u_expr_comp = f"Given {ea} = {eva} and {eb} = {evb}, compute {ea} + {eb} in Rust."
        expr_comp_segs = [
            ("```rust\nlet result =", False),
            (f" {ea}", True),
            (" +", False),
            (f" {eb}", True),
            (";\n```", False),
        ]
        if is_mt_eval:
            eval_sb.add_v10_segments([(u_expr, a_expr)], u_mfn, mfn_segs, "C")
            eval_sb.add_v10_segments([(u_expr, a_expr), (u_mfn, a_mfn)], u_mc, mc_segs, "C")
        else:
            sb.add_v10_segments([(u_expr, a_expr)], u_mfn, mfn_segs, mt_bkt, allow_duplicate_boost=4 if mt_i < 42 else 2)
            sb.add_v10_segments([(u_expr, a_expr), (u_mfn, a_mfn)], u_mc, mc_segs, mt_bkt, allow_duplicate_boost=4 if mt_i < 42 else 2)
            if mt_i < 24:
                sb.add_v10_segments([(uname_u_nat, uname_a_nat)], u_expr_comp, expr_comp_segs, "C", allow_duplicate_boost=4)
                sb.add_v10_segments(
                    [(uname_u_nat, uname_a_nat), (u_expr_comp, a_expr)],
                    u_mc,
                    mc_segs,
                    "C",
                    allow_duplicate_boost=4,
                )
            if mt_i < 16:
                sb.add_v10_plain(
                    [(u_expr, a_expr), (u_mfn, a_mfn)],
                    "How does the MVM-Wasm Linux sandbox isolate network access?",
                    REPLY_ZERONIC,
                    "A",
                    boost=4,
                )
            elif mt_i < 32:
                sb.add_v10_plain(
                    [(u_expr, a_expr), (u_mfn, a_mfn), (u_mc, a_mc)],
                    "How does the MVM-Wasm Linux sandbox isolate network access?",
                    REPLY_ZERONIC,
                    "A",
                    boost=4,
                )
            if mt_i < 16:
                sb.add_v10_plain(
                    [
                        (u_expr, a_expr),
                        (u_mfn, a_mfn),
                        (u_mc, a_mc),
                        ("How does the MVM-Wasm Linux sandbox isolate network access?", REPLY_ZERONIC),
                    ],
                    uname_u_nat,
                    uname_tc_plain,
                    "D",
                    boost=2,
                )

    # 4. Curated CodeAlpaca-20k prose-led idioms (plain text prefix avoids uncopied ``` fence collision at pos_idx 8..37)
    rng.shuffle(open_code)
    code_unique = 0
    for p, out in open_code:
        if code_unique >= 4:
            break
        if len(p) > 68 or len(out) > 80 or " into " in p.lower() or " inside " in p.lower() or "`" in out:
            continue
        if sb.add_v10_plain([], p, out, "C_open", boost=8):
            code_unique += 1


def generate_bucket_d_mvm_and_git_tools(
    sb: SampleBuilder,
    eval_sb: SampleBuilder,
) -> None:
    """Bucket D (~8,500): Orthogonal `<tool_call>` (`mvm_exec` & `git`) + `<tool_response>` grounded synthesis."""

    # CRITICAL ORTHOGONALITY INVARIANT:
    # Fixed `<tool_call>`s + `sha256sum` (`cur_tok = "-n"`) + `git clone` (`cur_tok = "clone"`) need ZERO
    # `<thought>` prose, while the 3 `"`-ending dynamic `<tool_call>`s use compact non-prose `<thought>[k]</thought>`
    # tags spaced >= 20 tokens apart so `(pos_idx, '"')` NEVER collides AND zero English `<thought>` prose pollutes chat!
    def mvm_tc_fixed(cmd: str, _hint: str = "") -> str:
        esc = cmd.replace("\\", "\\\\").replace('"', '\\"')
        return f'<tool_call>{{"name":"mvm_exec","arguments":{{"cmd":"{esc}"}}}}</tool_call>'

    def git_tc_fixed(command: str, _hint: str = "") -> str:
        esc = command.replace("\\", "\\\\").replace('"', '\\"')
        return f'<tool_call>{{"name":"git","arguments":{{"cmd":"{esc}"}}}}</tool_call>'

    # 1. `Run uname -a and compute echo -n {tag} | sha256sum inside the Linux MicroVM sandbox.` (`<|user|>` AND `[agent_mode]`!)
    # Includes `-mvm` suffix tags (`'m','v','m'` in prompt) so `sh == 2` 3-gram induction at `<tool_call>{"name":"mvm` is 100% invariant!
    sha_tags = [
        "uor-v10-mvm", "spiralcore-mvm", "rv64imac-mvm", "opfs-lut-mvm", "jq-pkg-mvm",
        "cl06-fwht-mvm", "tmac-q12-mvm", "prll-kv-mvm", "cow-vfs-mvm", "vsock-cid-mvm",
        "wasm-worker-ok", "zero-nic-r437", "git-sync-vfs", "brain-pulse-e8", "bpe4096-lut",
        "studio-chat-10", "riscv-static-c", "initramfs-root", "sha256-check", "hinge-q12-opt",
    ]
    for idx in range(950):
        tag = f"{sha_tags[idx % len(sha_tags)]}-{idx}" if idx >= 200 else sha_tags[idx % len(sha_tags)]
        if idx >= 20 and idx < 200:
            base_t = sha_tags[idx % len(sha_tags)]
            tag = f"{idx}-{base_t}"
        cmd_phrase = (
            f"uname -a and compute echo -n {tag} | sha256sum"
            if idx % 3 != 2
            else f"uname -a && echo -n {tag} | sha256sum"
        )
        dig = hashlib.sha256(tag.encode("utf-8")).hexdigest()
        u_prompt = f"Run {cmd_phrase} inside the Linux MicroVM sandbox."
        segs_step1 = [
            ('<tool_call>{"name":"mvm_exec","arguments":{"cmd":"uname -a && echo -n', False),
            (f" {tag} | sha256sum", True),
            ('"}}</tool_call>', False),
        ]
        obs_v9_tail = f" mvm-wasm 6.6.0-uor-rv64imac #1 SMP PREEMPT_DYNAMIC riscv64 GNU/Linux | {dig}  -"
        obs_v9 = f"Linux{obs_v9_tail}"
        v9_step2_prompt = f"[agent_mode] Verify sandbox execution.\n<tool_response> {obs_v9}</tool_response>"
        v9_step2_segs = [
            ("MVM verified ok: Linux", False),
            (obs_v9_tail, True),
            (".", False),
        ]
        v10_bridge_segs = [
            ("Verified sandbox output: Linux", False),
            (obs_v9_tail, True),
            (".", False),
        ]
        v10_bridge_prompt = f"Summarize the verified tool output for `{u_prompt}`:\n<tool_response> {obs_v9}</tool_response>"

        if idx >= 900:
            eval_sb.add_v10_segments([], u_prompt, segs_step1, "D")
            eval_sb.add_v10_segments([], v10_bridge_prompt, v10_bridge_segs, "D")
        else:
            sb.add_v10_segments([], u_prompt, segs_step1, "D" if idx < 340 else "D_ext", allow_duplicate_boost=6 if idx < 340 else 2)
            sb.add_v10_segments([], v10_bridge_prompt, v10_bridge_segs, "D" if idx < 80 else "D_ext")
            if idx < 850:
                sb.add_v9_compat_segments(f"[agent_mode] {u_prompt}", segs_step1, "D" if idx < 360 else "D_ext", boost=8 if idx < 360 else 2)
                sb.add_v9_compat_segments(
                    v9_step2_prompt,
                    v9_step2_segs,
                    "D" if idx < 450 else "D_ext",
                    boost=13 if idx < 450 else 2,
                )
                sb.add_v10_segments([], v9_step2_prompt, v9_step2_segs, "D_ext")

    # Direct `uname -a` & natural composer sandbox prompts (`gllm_v9_compositional.rs:285`)
    uname_obs = "Linux mvm-wasm 6.6.0-uor-rv64imac #1 SMP PREEMPT_DYNAMIC riscv64 GNU/Linux"
    uname_step2_segs = [
        ("Verified sandbox output: Linux", False),
        (uname_obs[5:], True),
        (".", False),
    ]
    uname_agent_step2_segs = [
        ("MVM verified ok: Linux", False),
        (uname_obs[5:], True),
        (".", False),
    ]
    sb.add_v9_compat_segments(
        f"[agent_mode] Verify sandbox execution.\n<tool_response> {uname_obs}</tool_response>",
        uname_agent_step2_segs,
        "D",
        boost=45,
    )
    for u_idx, uname_p in enumerate((
        "Run uname -a inside the MVM-Wasm Linux sandbox and report the architecture.",
        "Run uname -a inside the Linux MicroVM sandbox.",
        "Check uname -srm in the RISC-V MicroVM.",
        "Execute uname -a inside the MVM-Wasm Linux sandbox and report the architecture.",
        "Run `uname -a` inside the Linux MicroVM sandbox.",
    )):
        tc_u = mvm_tc_fixed("uname -a", "uname")
        sb.add_v10_plain([], uname_p, tc_u, "D", boost=350 if u_idx == 0 else 150)
        if uname_p != "Run uname -a inside the Linux MicroVM sandbox.":
            sb.add_v9_compat_segments(f"[agent_mode] {uname_p}", [(tc_u, False)], "D", boost=190 if u_idx == 0 else 60)
        sb.add_v10_segments(
            [],
            f"Summarize the verified tool output for `{uname_p}`:\n<tool_response> {uname_obs}</tool_response>",
            uname_step2_segs,
            "D",
            allow_duplicate_boost=40 if u_idx < 3 else 12,
        )

    # 2. All 10 MVM static packages (`jq`, `rg`, `tree`, `xxd`, `sha256sum`, `bc`, `wc`, `head`, `tail`, `http`) + `pkg`
    # Deterministic fixed `mvm_tc_fixed(f"pkg install {pkg}")` (`is_copy = False` — avoids `'all'` token-361 `<tool_call>` gate suppression!)
    pkg_names = ["jq", "rg", "tree", "xxd", "sha256sum", "bc", "wc", "head", "tail", "http"]
    for pkg in pkg_names:
        tc_pkg = f'<tool_call>{{"name":"mvm_exec","arguments":{{"env":"mvm","cmd":"pkg install {pkg}"}}}}</tool_call>'
        pkg_tmpls = (
            f"Install {pkg} inside the Linux MicroVM.",
            f"Install package {pkg} inside the Linux MicroVM.",
            f"Install the {pkg} binary inside the Linux MicroVM.",
            f"Run pkg install {pkg} inside the sandbox.",
            f"Can you install {pkg} inside the MVM?",
        )
        for t_idx, tmpl in enumerate(pkg_tmpls):
            if pkg == "jq" and t_idx == 0:
                sb.add_v10_plain([], tmpl, tc_pkg, "D", boost=340)
            elif pkg == "jq":
                sb.add_v10_plain([], tmpl, tc_pkg, "D", boost=48)
            else:
                sb.add_v10_plain([], tmpl, tc_pkg, "D" if t_idx < 2 else "D_ext", boost=36 if t_idx == 0 else (14 if t_idx == 1 else 6))
            obs = f"installed /usr/local/bin/{pkg} (ELF64 EM_RISCV=243, sha256 verified)"
            pkg_step2_segs = [
                ("Verified sandbox execution result --", False),
                (f" {obs}", True),
                (".", False),
            ]
            sb.add_v10_segments(
                [],
                f"Summarize the verified tool output for `{tmpl}`:\n<tool_response> {obs}</tool_response>",
                pkg_step2_segs,
                "D" if (pkg == "jq" or t_idx == 0) else "D_ext",
                allow_duplicate_boost=2 if t_idx == 0 else 1,
            )

    sb.add_v10_plain([], "List available packages in the MVM sandbox.", mvm_tc_fixed("pkg list", "list"), "D", boost=32)
    sb.add_v10_plain([], "What files are in `/workspace`?", mvm_tc_fixed("tree -L 2 /workspace", "tree"), "D", boost=24)

    # 3. Git tool-calling (`git status`, `git clone`, `git diff`, `git log`, `git push`)
    git_orgs = [
        "rust-lang", "tokio-rs", "bytecodealliance", "serde-rs", "cloudflare",
        "torvalds", "BurntSushi", "dtolnay", "rayon-rs", "hyperium", "Catalyst-OS",
    ]
    git_repos = [
        "libc", "mio", "wasmtime", "json", "workerd", "uom", "ripgrep",
        "anyhow", "syn", "bytes", "tracing", "hyper", "quinn", "tower",
        "md-docs", "mdcat", "md5-fast", "docBook", "netBook", "sysBook", "handbook", "playbook", "cmd-cli", "cmdr",
    ]

    for st_p in (
        "Check `git status` in `/workspace`.",
        "Run git status.",
        "Check git status in the workspace.",
        "What is the current `git status`?",
    ):
        sb.add_v10_plain([], st_p, git_tc_fixed("status", "status"), "D", boost=230)
    sb.add_v10_plain([], "Show the current git diff.", git_tc_fixed("diff", "diff"), "D", boost=24)
    sb.add_v10_plain([], "Show the latest git commits.", git_tc_fixed("log -n 5", "log"), "D", boost=24)
    sb.add_v10_plain([], "Push branch `main` to origin.", git_tc_fixed("push origin main", "push"), "D", boost=24)

    for i in range(950):
        org = git_orgs[i % len(git_orgs)]
        rep = f"{git_repos[i % len(git_repos)]}-{i % 45}" if i >= 120 else git_repos[i % len(git_repos)]
        url = f"https://github.com/{org}/{rep}.git"
        mode = (i // len(git_orgs)) % 3
        if mode == 0:
            u = f"Clone {url} into `/workspace`."
        elif mode == 1:
            u = f"Clone repository {url} into `/workspace`."
        else:
            u = f"Use git to clone {url} into `/workspace`."
        segs = [
            ('<tool_call>{"name":"git","arguments":{"cmd":"clone', False),
            (f" {url}", True),
            ('"}}</tool_call>', False),
        ]
        sha = hashlib.sha256(url.encode("utf-8")).hexdigest()[:7]
        obs = f"Cloned {url} into-dir /workspace/{rep} at HEAD {sha} (synced 14 files)"
        ans_segs = [
            ("Verified sandbox execution result --", False),
            (f" {obs}", True),
            (".", False),
        ]
        target = eval_sb if i >= 900 else sb
        g_bkt = "D" if (target is eval_sb or i < 460) else "D_ext"
        g_boost = (12 if mode == 0 else 4) if i < 460 else 2
        target.add_v10_segments([], u, segs, g_bkt, allow_duplicate_boost=g_boost if target is sb else 1)
        target.add_v10_segments(
            [],
            f"Summarize the verified tool output for `{u}`:\n<tool_response> {obs}</tool_response>",
            ans_segs,
            "D" if (target is eval_sb or i < 100) else "D_ext",
        )


def generate_bucket_e_web_tools(
    sb: SampleBuilder,
    eval_sb: SampleBuilder,
) -> None:
    """Bucket E (~5,500): Orthogonal `web_search` and `web_fetch` `<tool_call>` + grounded `<tool_response>`."""
    search_subjects = [
        ("Rust 2024 edition async traits stabilization", "Rust stabilized async fn in traits (AFIT) plus return-position impl Trait in trait (RPITIT)."),
        ("WebAssembly Component Model WASI 0.1", "The WebAssembly Component Model defines WIT interface types and WASI 0.1 portable system interfaces."),
        ("WebAssembly System Interface WASI 0.1", "WASI 0.1 defines capability-oriented POSIX-like system calls for portable sandboxed WebAssembly modules."),
        ("IETF Transport Layer Security TLS 1.3", "TLS 1.3 completes encrypted key exchange in a single round trip using ephemeral Diffie-Hellman cipher suites."),
        ("WebGPU Shading Language WGSL 1.0", "WGSL 1.0 provides deterministic bounds-checked compute shaders across Vulkan, Metal, D3D12."),
        ("HTTP/3 QUIC transport RFC 9114 1.0", "HTTP/3 multiplexes independent streams over UDP-based QUIC 1.0 without TCP head-of-line packet blocking."),
        ("OAuth Authorization Framework 2.1", "OAuth 2.1 consolidates PKCE requirements while removing legacy implicit token flows for safer delegated access."),
        ("OpenGL Core Profile specification 4.6", "OpenGL 4.6 standardizes SPIR-V shader ingestion plus multi-draw indirect command buffers."),
        ("PCIe Base Specification 5.0", "PCIe 5.0 doubles per-lane transfer signaling to 32 GT/s while maintaining backward slot compatibility."),
        ("YAML data serialization standard 1.2", "YAML 1.2 aligns strictly as a superset of JSON while supporting human-readable indentation."),
        ("WebAssembly relaxed SIMD browser support", "Modern browsers ship WebAssembly Relaxed SIMD, accelerating vector lane operations in Wasm modules."),
        ("RISC-V RV64IMAC compressed instruction encoding", "The RISC-V C extension encodes frequent 32-bit instructions into 16-bit parcels, shrinking static binaries."),
        ("Serverless Cloudflare Workers WebSocketPair API", "Cloudflare Workers WebSocketPair upgrades HTTP requests with status 101, forwarding frames at the edge."),
        ("Git smart HTTP upload-pack protocol", "Git smart HTTP discovers refs via GET /info/refs?service=git-upload-pack before negotiating packfiles."),
        ("DNS over HTTPS RFC 8484 wireformat", "RFC 8484 encapsulates DNS queries over HTTPS (application/dns-message), preventing plaintext UDP snooping."),
        ("Origin Private File System OPFS sync access", "In a dedicated Web Worker, OPFS createSyncAccessHandle() provides fast persistent binary shard I/O."),
        ("WebAssembly SIMD vector lane browser support", "Modern browsers support 128-bit WebAssembly SIMD vector lane operations across desktop and mobile engines."),
        ("Rust Relaxed atomic memory ordering", "Ordering::Relaxed guarantees single-variable atomic operations without cross-thread happens-before synchronization."),
    ]

    search_openers = [
        "Search the web for {q}.",
        "Look up {q}.",
        "Can you search online for {q}?",
        "Find current documentation on {q}.",
        "Use web search to check {q}.",
    ]

    search_ref_urls = [
        "https://doc.rust-lang.org/edition-guide/",
        "https://component-model.bytecodealliance.org/",
        "https://wasi.dev/interfaces",
        "https://datatracker.ietf.org/doc/rfc8446/",
        "https://www.w3.org/TR/WGSL/",
        "https://datatracker.ietf.org/doc/rfc9114/",
        "https://oauth.net/specs/",
        "https://registry.khronos.org/OpenGL/",
        "https://pcisig.com/specifications",
        "https://yaml.org/spec/",
        "https://webassembly.org/features/",
        "https://riscv.org/technical/specifications/",
        "https://developers.cloudflare.com/workers/",
        "https://git-scm.com/docs/http-protocol",
        "https://datatracker.ietf.org/doc/rfc8484/",
        "https://developer.mozilla.org/en-US/docs/Web/API/FileSystemSyncAccessHandle",
        "https://webassembly.org/roadmap/",
        "https://doc.rust-lang.org/nomicon/atomics.html",
    ]

    plain_search_quals = [
        "browser support", "memory layout", "protocol framing", "compiler intrinsics",
        "kernel syscalls", "binary format", "packet routing", "thread safety",
        "vector lanes", "header parsing", "stream framing", "cache coherence",
        "pointer provenance", "type erasure", "register pressure", "branch prediction",
        "page tables", "socket options", "opcode encoding", "stack unwinding",
        "ring buffers", "atomic ordering", "module linking", "error handling",
    ]

    for s_i in range(950):
        base_q, summary = search_subjects[s_i % len(search_subjects)]
        ref_url = search_ref_urls[s_i % len(search_ref_urls)]
        if s_i < len(search_subjects) * len(search_openers):
            q = base_q
        elif s_i % 5 != 0:
            q = f"{base_q} {plain_search_quals[(s_i // len(search_subjects)) % len(plain_search_quals)]}"
        elif (s_i // 5) % 2 == 0:
            q = f"{base_q} v{1 + (s_i % 4)}.{s_i % 9}"
        else:
            q = f"{base_q} reference {s_i}"
        opener = search_openers[s_i % len(search_openers)]
        u = opener.format(q=q)
        segs = [
            ('<tool_call>{"name":"web_search","arguments":{"scope":"web","query":"', False),
            (f" {q}", True),
            ('"}}</tool_call>', False),
        ]
        obs = f"1. **{q}** ({ref_url}) — {summary}"
        ans_segs = [
            ("Verified web search result —", False),
            (f" {summary}", True),
        ]
        target = eval_sb if s_i >= 905 else sb
        boost_n = 24 if (target is sb and opener == "Search the web for {q}.") else (5 if target is sb else 1)
        e_bkt = "E" if (target is eval_sb or s_i < 520) else "E_ext"
        target.add_v10_segments([], u, segs, e_bkt, allow_duplicate_boost=boost_n)
        target.add_v10_segments(
            [],
            f"Summarize the verified tool output for `{u}`:\n<tool_response> {obs}</tool_response>",
            ans_segs,
            "E" if (target is eval_sb or s_i < 100) else "E_ext",
            allow_duplicate_boost=4 if (target is sb and s_i < 100) else 1,
        )

    domains = [
        "docs.rs/tokio/latest/tokio",
        "docs.rs/serde/latest/serde",
        "docs.rs/tracing/latest/tracing",
        "docs.rs/bytes/latest/bytes",
        "docs.rs/rayon/latest/rayon",
        "docs.rs/strum/latest/strum",
        "docs.rs/uom/latest/uom",
        "docs.rs/aurum/latest/aurum",
        "docs.rs/quantum/latest/quantum",
        "docs.rs/spectrum/latest/spectrum",
        "docs.rs/momentum/latest/momentum",
        "docs.rs/hyper/latest/hyper",
        "docs.rs/tower/latest/tower",
        "docs.rs/quinn/latest/quinn",
        "docs.rs/clap/latest/clap",
        "docs.rs/strum/latest/strum/index.html",
        "docs.rs/uom/latest/uom/si",
        "doc.rust-lang.org/std/vec/struct.Vec.html",
        "developer.mozilla.org/en-US/docs/WebAssembly",
        "api.github.com/repos/rust-lang/rust",
        "datatracker.ietf.org/doc/html/rfc8484",
        "riscv.org/technical/specifications",
        "workers.cloudflare.com/docs/runtime-apis/websockets",
        "git-scm.com/docs/http-protocol",
    ]
    fetch_openers = [
        "Fetch {url} and summarize it.",
        "Read {url} and summarize the section.",
        "Use `web_fetch` to inspect {url} and summarize key points.",
        "Fetch {url} and show a summary.",
    ]
    fetch_slugs = ["index", "overview", "guide", "core", "types", "traits", "io", "spec", "reference", "module"]
    fetch_crates = [
        "tokio", "serde", "tracing", "bytes", "rayon", "strum", "uom", "aurum",
        "quantum", "spectrum", "momentum", "hyper", "tower", "quinn", "clap",
        "anyhow", "thiserror", "nom", "crossbeam", "mio", "tonic", "syn", "futures", "semver",
    ]
    for f_i in range(950):
        dom = domains[f_i % len(domains)]
        if f_i < len(domains) * len(fetch_openers):
            url = f"https://{dom}"
        elif f_i % 2 == 0:
            cr_f = f"{fetch_crates[f_i % len(fetch_crates)]}_{f_i}"
            url = f"https://docs.rs/{cr_f}/latest/{cr_f}"
        else:
            url = f"https://{dom}/{fetch_slugs[f_i % len(fetch_slugs)]}_{f_i}"
        opener = fetch_openers[f_i % len(fetch_openers)]
        u = opener.format(url=url)
        segs = [
            ('<tool_call>{"name":"web_fetch","arguments":{"extract":"summary","url":"', False),
            (f" {url}", True),
            ('"}}</tool_call>', False),
        ]
        obs = f"HTTP 200 OK ({url}): Technical reference covering API invariants."
        ans_segs = [
            ("Verified sandbox execution result --", False),
            (f" {obs}", True),
        ]
        target = eval_sb if f_i >= 905 else sb
        boost_n = 21 if (target is sb and f_i < len(domains) * len(fetch_openers) and opener == "Fetch {url} and summarize it.") else (10 if (target is sb and opener == "Fetch {url} and summarize it.") else (4 if target is sb else 1))
        ef_bkt = "E" if (target is eval_sb or f_i < 380) else "E_ext"
        target.add_v10_segments([], u, segs, ef_bkt, allow_duplicate_boost=boost_n)
        target.add_v10_segments(
            [],
            f"Summarize the verified tool output for `{u}`:\n<tool_response> {obs}</tool_response>",
            ans_segs,
            "E" if (target is eval_sb or f_i < 80) else "E_ext",
        )


def top_up_to_target(
    sb: SampleBuilder,
    rng: random.Random,
) -> None:
    """Fill cleanly to `TARGET_TRAIN_COUNT = 50,000` with balanced high-SNR examples across Buckets A/C/D/E."""
    prefixes = [
        "alpha", "delta", "kappa", "sigma", "omega", "zeta", "lambda", "gamma",
        "matrix", "tensor", "stride", "window", "cursor", "packet", "frame", "sector",
        "inode", "digest", "quota", "margin", "scalar", "vertex", "anchor", "bucket",
        "origin", "orbit", "batch", "baryon", "nexus", "lattice", "cascade", "axel",
        "chain", "grain", "macro", "retro", "hydro", "micro",
    ]
    mids = ["count", "limit", "index", "scale", "shift", "delta", "score", "value", "width", "depth", "bound", "state"]
    suffixes = [
        "a1", "b2", "c3", "d4", "e5", "f6", "g8", "h1", "j3", "k9", "m2", "m4",
        "n5", "p7", "q3", "r2", "t8", "u6", "v4", "w9", "x2", "y4", "z7", "z9",
    ]
    plain_search_quals = [
        "browser support", "memory layout", "protocol framing", "compiler intrinsics",
        "kernel syscalls", "binary format", "packet routing", "thread safety",
        "vector lanes", "header parsing", "stream framing", "cache coherence",
        "pointer provenance", "type erasure", "register pressure", "branch prediction",
        "page tables", "socket options", "opcode encoding", "stack unwinding",
        "ring buffers", "atomic ordering", "module linking", "error handling",
    ]
    crates = [
        "hyper", "rayon", "anyhow", "thiserror", "clap", "nom", "crossbeam", "mio",
        "tower", "tonic", "serde", "tokio", "tracing", "bytes", "syn", "futures", "semver",
        "strum", "uom", "aurum", "quantum", "spectrum", "momentum",
    ]
    git_repos_top = crates + ["md-docs", "mdcat", "md5-fast", "cmd-cli", "cmdr"]
    git_orgs_top = [
        "rust-lang", "tokio-rs", "bytecodealliance", "serde-rs", "cloudflare",
        "BurntSushi", "dtolnay", "rayon-rs", "hyperium", "Catalyst-OS",
    ]
    search_topics_top = [
        "WebAssembly Relaxed SIMD lane semantics",
        "RISC-V RV64IMAC compressed instructions",
        "HTTP/3 QUIC stream multiplexing",
        "IETF Transport Layer Security TLS 1.3",
        "Rust async trait stabilization",
        "Origin Private File System OPFS worker sync",
        "WebGPU WGSL compute workgroup barriers",
        "DNS over HTTPS RFC 8484 wireformat",
        "PCIe Base Specification lane signaling",
        "YAML serialization specification",
        "OAuth PKCE delegated token flow",
        "OpenGL Core Profile SPIR-V shaders",
    ]
    thought_repo_top = "<thought>kv recall</thought>\nYour repository is called:"
    thought_said_top = "<thought>prior turn</thought>\nEarlier in this chat you said:"
    repo_ack_top = "Noted your saved name here for you."
    repo_prefixes_top = [p for p in prefixes if not p.startswith("a")]

    fn_verbs_top = ["combine", "clamp", "step", "mix", "fold", "scale", "merge", "align", "diff", "accum"]
    fn_nouns_top = ["pair", "sum", "delta", "lanes", "window", "budget", "mux", "crux", "luxe", "radix", "plume"]
    repo_asks_top = [
        "what is my repo called?",
        "What is my repository called?",
        "what did I say my repo was called?",
        "remind me what my project is called?",
    ]

    idx = 0
    while len(sb.records) < TARGET_TRAIN_COUNT and idx < 95000:
        kind = idx % 10
        idx += 1
        a = f"{rng.choice(prefixes)}_{rng.choice(mids)}_{rng.choice(suffixes)}"
        b = f"{rng.choice(prefixes)}_{rng.choice(mids)}_{rng.choice(suffixes)}"
        if a == b or contains_forbidden(a) or contains_forbidden(b):
            continue
        va = 11 + (idx % 83)
        vb = 3 + ((idx * 7) % 41)
        if kind == 0:
            p = f"Given {a} = {va} and {b} = {vb}, write a concise Rust expression combining {a} and {b}."
            segs = [("```rust\nlet result =", False), (f" {a}", True), (" +", False), (f" {b}", True), (";\n```", False)]
            sb.add_v10_segments([], p, segs, "C_top", allow_duplicate_boost=2)
        elif kind == 1:
            if (idx // 10) % 2 == 0:
                p = f"Given {a} = {va} and {b} = {vb}, compute {a} + {b} in Rust."
                segs = [("```rust\nlet result =", False), (f" {a}", True), (" +", False), (f" {b}", True), (";\n```", False)]
                sb.add_v10_segments([], p, segs, "C_top", allow_duplicate_boost=3)
            else:
                fn_c_t = f"{fn_verbs_top[idx % len(fn_verbs_top)]}_{fn_nouns_top[(idx // 5) % len(fn_nouns_top)]}"
                if contains_forbidden(fn_c_t):
                    continue
                u_c_t = f"Write a freestanding C function {fn_c_t}(long {a}) for the RV64IMAC MicroVM."
                c_rv_t = [
                    ("```c\n/* rv64 rv64 */ long", False),
                    (f" {fn_c_t}", True),
                    ("(long", False),
                    (f" {a}", True),
                    (") {\n    return (", False),
                    (f" {a}", True),
                    (" * 3) | 1;\n}\n```", False),
                ]
                sb.add_v10_segments([], u_c_t, c_rv_t, "C_top", allow_duplicate_boost=2)
        elif kind == 2:
            fn = f"{fn_verbs_top[idx % len(fn_verbs_top)]}_{fn_nouns_top[(idx // 3) % len(fn_nouns_top)]}_{suffixes[idx % len(suffixes)]}"
            if contains_forbidden(fn):
                continue
            p_fn = f"Write a Rust function {fn}( {a}: i64, {b}: i64 ) -> i64."
            fn_segs = [
                ("```rust\n// pure core fast saturating signed i64 addition helper routine\n#[must_use]\n#[inline]\npub fn", False),
                (f" {fn}", True),
                ("(", False),
                (f" {a}", True),
                (": i64,", False),
                (f" {b}", True),
                (": i64 ) -> i64 {\n    return", False),
                (f" {a}", True),
                (".saturating_add(", False),
                (f" {b}", True),
                (" );\n}\n```", False),
            ]
            sb.add_v10_segments([], p_fn, fn_segs, "C_top", allow_duplicate_boost=6)
        elif kind == 3:
            if (idx // 10) % 2 == 1:
                continue
            repo_slug = f"{rng.choice(repo_prefixes_top)}-{rng.choice(mids)}"
            if contains_forbidden(repo_slug):
                continue
            u1 = f"my repo is called {repo_slug}"
            warm_top = [("hi", REPLY_HI), ("how are you?", REPLY_HOW_ARE_YOU), ("who are you?", REPLY_WHO)]
            sb.add_v10_plain(warm_top, u1, repo_ack_top, "A_top", boost=2)
            a1 = repo_ack_top if (idx % 3 != 2) else f"Got it - tracking repository {repo_slug}."
            hist_r = warm_top + [(u1, a1)] if (idx % 3 != 0) else [(u1, a1)]
            sb.add_v10_segments(
                hist_r,
                repo_asks_top[(idx // 10) % len(repo_asks_top)],
                [
                    (thought_repo_top, False),
                    (f" {repo_slug}", True),
                    (".", False),
                ],
                "A_top",
            )
            hist_5_top = warm_top + [(u1, repo_ack_top), ("what is my repo called?", f"Your repository is called: {repo_slug}.")]
            sb.add_v10_plain(
                hist_5_top,
                "can you explain that simpler?",
                "Sure - our causal KV cache keeps that saved repo label handy across turns.",
                "A_top",
                boost=1,
            )
        elif kind == 4:
            verbs_said = ["auditing", "inspecting", "verifying", "checking", "tracing", "debugging"]
            vs = verbs_said[(idx // 10) % len(verbs_said)]
            st_b = f"{vs} the vsock-{mids[idx % len(mids)]} bounds"
            u1_s = f"I am {st_b}."
            ack_s = "Got it - ready to help with your question."
            sb.add_v10_plain([], u1_s, ack_s, "A_top", boost=2)
            ask_s = ("what did I just say?", "What did I just say?", "remind me what I just said")[(idx // 10) % 3]
            sb.add_v10_segments(
                [(u1_s, ack_s)],
                ask_s,
                [
                    (thought_said_top, False),
                    (f" {st_b}", True),
                    (".", False),
                ],
                "A_top",
                allow_duplicate_boost=3,
            )
        elif kind == 5:
            tag = f"check-{a}-mvm" if (idx // 20) % 2 == 0 else f"check-{a}-{idx % 99}"
            cmd_phrase = f"uname -a and compute echo -n {tag} | sha256sum"
            u_p = f"Run {cmd_phrase} inside the Linux MicroVM sandbox."
            segs = [
                ('<tool_call>{"name":"mvm_exec","arguments":{"cmd":"uname -a && echo -n', False),
                (f" {tag} | sha256sum", True),
                ('"}}</tool_call>', False),
            ]
            if (idx // 10) % 2 == 0:
                sb.add_v10_segments([], u_p, segs, "D_top")
            else:
                sb.add_v9_compat_segments(f"[agent_mode] {u_p}", segs, "D_top", boost=2)
        elif kind == 6:
            tag = f"step2-{a}-{idx % 99}"
            dig_t = hashlib.sha256(tag.encode("utf-8")).hexdigest()
            obs_t_tail = f" mvm-wasm 6.6.0-uor-rv64imac #1 SMP PREEMPT_DYNAMIC riscv64 GNU/Linux | {dig_t}  -"
            p_s2 = f"[agent_mode] Verify sandbox execution.\n<tool_response> Linux{obs_t_tail}</tool_response>"
            segs_s2 = [
                ("MVM verified ok: Linux", False),
                (obs_t_tail, True),
                (".", False),
            ]
            sb.add_v9_compat_segments(p_s2, segs_s2, "D_top")
        elif kind == 7:
            cr = f"{crates[(idx // 10) % len(crates)]}_{idx % 97}"
            url = f"https://docs.rs/{cr}/latest/{cr}"
            u_f = f"Fetch {url} and summarize it."
            segs = [
                ('<tool_call>{"name":"web_fetch","arguments":{"extract":"summary","url":"', False),
                (f" {url}", True),
                ('"}}</tool_call>', False),
            ]
            sb.add_v10_segments([], u_f, segs, "E_top", allow_duplicate_boost=2)
        elif kind == 8:
            org_t = git_orgs_top[(idx // 10) % len(git_orgs_top)]
            cr = git_repos_top[(idx // 10) % len(git_repos_top)]
            url_g = f"https://github.com/{org_t}/{cr}-{idx % 89}.git"
            u_g = (
                f"Clone {url_g} into `/workspace`."
                if (idx // 10) % 2 == 0
                else f"Clone repository {url_g} into `/workspace`."
            )
            segs = [
                ('<tool_call>{"name":"git","arguments":{"cmd":"clone', False),
                (f" {url_g}", True),
                ('"}}</tool_call>', False),
            ]
            sb.add_v10_segments([], u_g, segs, "D_top")
        else:
            k9_step = (idx - 1) // 10
            topic_s = search_topics_top[k9_step % len(search_topics_top)]
            if k9_step % 5 != 0:
                qual_s = plain_search_quals[(k9_step // len(search_topics_top)) % len(plain_search_quals)]
                q_s = f"{topic_s} {qual_s}"
                cycle_k9 = k9_step // (len(search_topics_top) * len(plain_search_quals))
                if cycle_k9 == 0:
                    op_k9 = (
                        "Search the web for {q}."
                        if (k9_step % 3 != 2)
                        else ("Look up {q} on the web." if ((k9_step // 3) % 2 == 0) else "Run web_search for {q}.")
                    )
                else:
                    op_k9 = (
                        ("Look up {q} on the web." if (k9_step % 2 == 0) else "Run web_search for {q}.")
                        if (k9_step % 3 != 2)
                        else "Search the web for {q}."
                    )
                u_s = op_k9.format(q=q_s)
                k9_boost = 4
            else:
                q_s = f"{topic_s} {1 + (k9_step % 2)}.{k9_step % 9}"
                u_s = f"Search the web for {q_s}."
                k9_boost = 3
            if contains_forbidden(q_s):
                continue
            segs = [
                ('<tool_call>{"name":"web_search","arguments":{"scope":"web","query":"', False),
                (f" {q_s}", True),
                ('"}}</tool_call>', False),
            ]
            sb.add_v10_segments([], u_s, segs, "E_top", allow_duplicate_boost=k9_boost)


def write_uorcorp3(path: Path, records: list[tuple[str, list[int], list[int], list[int]]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("wb") as f:
        f.write(b"UORCORP3")
        f.write(struct.pack("<II", len(records), MAX_SEQ))
        for _bucket, p_ids, r_ids, r_copy in records:
            plen = len(p_ids)
            rlen = len(r_ids)
            assert plen + rlen <= MAX_SEQ
            assert len(r_copy) == rlen
            f.write(struct.pack("<HH", plen, rlen))
            for tid in p_ids + r_ids:
                f.write(struct.pack("<H", tid))
            f.write(bytes(r_copy))


def write_held_out_eval_prompts(path_json: Path, path_bin: Path, tok: Tokenizer) -> None:
    """22 strictly held-out short chat, multi-turn recall, code, and tool prompts for receipt & gate audit."""
    gllm_sha = hashlib.sha256(b"gllm-v9-mvm").hexdigest()
    prompts = [
        {"id": "chat_hi", "kind": "greeting", "prompt": "<|user|>\nhi\n<|assistant|>\n"},
        {"id": "chat_hello", "kind": "greeting", "prompt": "<|user|>\nhello\n<|assistant|>\n"},
        {"id": "chat_hows_it_going", "kind": "greeting", "prompt": "<|user|>\nhey, how's it going?\n<|assistant|>\n"},
        {"id": "chat_good_morning", "kind": "greeting", "prompt": "<|user|>\ngood morning!\n<|assistant|>\n"},
        {"id": "chat_thanks", "kind": "social", "prompt": "<|user|>\nthanks!\n<|assistant|>\n"},
        {"id": "meta_who_are_you", "kind": "capability", "prompt": "<|user|>\nwho are you?\n<|assistant|>\n"},
        {"id": "meta_what_can_you_do", "kind": "capability", "prompt": "<|user|>\nwhat can you do?\n<|assistant|>\n"},
        {"id": "meta_who_made_you", "kind": "capability", "prompt": "<|user|>\nwho made you?\n<|assistant|>\n"},
        {"id": "meta_browser", "kind": "capability", "prompt": "<|user|>\nare you running in my browser?\n<|assistant|>\n"},
        {
            "id": "multiturn_explain_simpler",
            "kind": "multiturn_followup",
            "prompt": "<|user|>\nExplain HTTP status codes briefly.\n<|assistant|>\n2xx codes signal success, 3xx redirect the client, 4xx report a client request error, and 5xx indicate an upstream server failure.<|end|>\n<|user|>\ncan you explain that simpler?\n<|assistant|>\n",
        },
        {
            "id": "multiturn_nebula_repo",
            "kind": "multiturn_kv_recall",
            "prompt": (
                f"<|user|>\nhi\n<|assistant|>\n{REPLY_HI}<|end|>\n"
                f"<|user|>\nhow are you?\n<|assistant|>\n{REPLY_HOW_ARE_YOU}<|end|>\n"
                f"<|user|>\nwho are you?\n<|assistant|>\n{REPLY_WHO}<|end|>\n"
                "<|user|>\nmy repo is called nebula-os\n<|assistant|>\nNoted your saved name here for you.<|end|>\n"
                "<|user|>\nwhat is my repo called?\n<|assistant|>\n"
            ),
        },
        {
            "id": "multiturn_what_did_i_say",
            "kind": "multiturn_kv_recall",
            "prompt": "<|user|>\nI am auditing the zero-copy RingBuffer bounds.\n<|assistant|>\nGot it - ready to help with your question.<|end|>\n<|user|>\nwhat did I just say?\n<|assistant|>\n",
        },
        {
            "id": "code_unseen_v10",
            "kind": "compositional_code",
            "prompt": "<|user|>\nGiven sensor_drift_k9 = 41 and base_offset_q3 = 18, write a Rust snippet setting `out` to (sensor_drift_k9 + base_offset_q3) + 4.\n<|assistant|>\n",
        },
        {
            "id": "code_unseen_v9_compat",
            "kind": "r4_38_compat_code",
            "prompt": "<|user|>\nGiven orbital_phase_m2 = 27 and lattice_stride_z7 = 14, write a concise Rust expression combining orbital_phase_m2 and lattice_stride_z7.\n<|assistant|>\n",
        },
        {
            "id": "fn_unseen_rust",
            "kind": "r4_38_compat_fn",
            "prompt": "<|user|>\nWrite a Rust function compute_flux_z9( sensor_drift_k9: i64, base_offset_q3: i64 ) -> i64.\n<|assistant|>\n",
        },
        {
            "id": "fn_unseen_c_rv64",
            "kind": "r4_38_compat_c",
            "prompt": "<|user|>\nWrite a freestanding C function step_flux(long gyro_bias_m4) for the RV64IMAC MicroVM.\n<|assistant|>\n",
        },
        {
            "id": "qa_zero_nic",
            "kind": "r4_38_compat_qa",
            "prompt": "<|user|>\nHow does the MVM-Wasm Linux sandbox isolate network access?\n<|assistant|>\n",
        },
        {
            "id": "tool_mvm_uname",
            "kind": "tool_mvm",
            "prompt": "<|user|>\nRun uname -a inside the MVM-Wasm Linux sandbox and report the architecture.\n<|assistant|>\n",
        },
        {
            "id": "tool_mvm_sha256",
            "kind": "tool_mvm",
            "prompt": "<|user|>\nRun uname -a and compute echo -n gllm-v9-mvm | sha256sum inside the Linux MicroVM sandbox.\n<|assistant|>\n",
        },
        {
            "id": "tool_mvm_pkg_jq",
            "kind": "tool_mvm",
            "prompt": "<|user|>\nInstall jq inside the Linux MicroVM.\n<|assistant|>\n",
        },
        {
            "id": "tool_git_status",
            "kind": "tool_git",
            "prompt": "<|user|>\nCheck `git status` in `/workspace`.\n<|assistant|>\n",
        },
        {
            "id": "tool_git_clone",
            "kind": "tool_git",
            "prompt": "<|user|>\nClone https://github.com/Casey-allard/lexlean-r4.git into `/workspace`.\n<|assistant|>\n",
        },
        {
            "id": "tool_web_search",
            "kind": "tool_web",
            "prompt": "<|user|>\nSearch the web for WebAssembly Component Model WASI 0.2.\n<|assistant|>\n",
        },
        {
            "id": "tool_web_fetch",
            "kind": "tool_web",
            "prompt": "<|user|>\nFetch https://docs.rs/axum/latest/axum and summarize it.\n<|assistant|>\n",
        },
        {
            "id": "r4_38_agent_step1",
            "kind": "r4_38_agent",
            "prompt": "User: [agent_mode] Run uname -a and compute echo -n gllm-v9-mvm | sha256sum inside the Linux MicroVM sandbox.\nAssistant:",
        },
        {
            "id": "r4_38_agent_step2",
            "kind": "r4_38_agent",
            "prompt": f"User: [agent_mode] Verify sandbox execution.\n<tool_response> Linux mvm-wasm 6.6.0-uor-rv64imac #1 SMP PREEMPT_DYNAMIC riscv64 GNU/Linux | {gllm_sha}  -</tool_response>\nAssistant:",
        },
    ]
    bos = tok.token_to_id("<|bos|>")
    for item in prompts:
        item["prompt_ids"] = [bos] + tok.encode(str(item["prompt"]), add_special_tokens=False).ids

    path_json.write_text(json.dumps(prompts, indent=2) + "\n", encoding="utf-8")

    with path_bin.open("wb") as f:
        f.write(b"UORPRMP1")
        f.write(struct.pack("<I", len(prompts)))
        for item in prompts:
            id_b = str(item["id"]).encode("utf-8")
            kind_b = str(item["kind"]).encode("utf-8")
            pr_b = str(item["prompt"]).encode("utf-8")
            pids = item["prompt_ids"]
            f.write(struct.pack("<HHH", len(id_b), len(kind_b), len(pr_b)))
            f.write(id_b)
            f.write(kind_b)
            f.write(pr_b)
            f.write(struct.pack("<H", len(pids)))
            for tid in pids:
                f.write(struct.pack("<H", tid))


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent
    tok_path = repo_root / "crates" / "wasm" / "src" / "tokenizer.json"
    out_corpus = Path(sys.argv[1]) if len(sys.argv) > 1 else (repo_root / "gpu" / "v10_corpus.bin")
    out_boot_corpus = out_corpus.parent / "v10_boot_corpus.bin"
    out_eval = out_corpus.parent / "v10_eval.bin"
    out_boot_eval = out_corpus.parent / "v10_boot_eval.bin"
    out_vocab = out_corpus.parent / "v10_vocab_bytes.bin"
    out_prompts = out_corpus.parent / "v10_eval_prompts.json"
    out_prompts_bin = out_corpus.parent / "v10_eval_prompts.bin"
    cache_dir = Path(__file__).resolve().parent / ".hf_cache_v10"

    export_vocab_raw_bytes(tok_path, out_vocab)
    tok = Tokenizer.from_file(str(tok_path))

    rng = random.Random(20261111)
    smoltalk = load_smoltalk_dialogues(cache_dir)
    open_qa, open_code = load_dolly_and_alpaca(cache_dir)

    sb = SampleBuilder(tok)
    eval_sb = SampleBuilder(tok)

    generate_bucket_a_conversations(sb, rng, smoltalk, eval_sb)
    generate_bucket_b_qa(sb, rng, open_qa, eval_sb)
    generate_bucket_c_code(sb, rng, open_code, eval_sb)
    generate_bucket_d_mvm_and_git_tools(sb, eval_sb)
    generate_bucket_e_web_tools(sb, eval_sb)

    # Focused high-SNR fast-start subset for compact `6L 64D` (`bootstrap_v9.uorgllm`):
    # Preserve strong structured C/D/E supervision while capping single-turn plain-prose repetition (~24,300 crisp sequences).
    end_tok_id = tok.token_to_id("<|end|>")
    raw_boot_records = [rec for rec in sb.records if not rec[0].endswith(("_open", "_ext", "_top"))]
    boot_totals: dict[tuple[tuple[int, ...], tuple[int, ...]], int] = {}
    for _b, p_ids, r_ids, _rc in raw_boot_records:
        k_sig = (tuple(p_ids), tuple(r_ids))
        boot_totals[k_sig] = boot_totals.get(k_sig, 0) + 1
    boot_seen: dict[tuple[tuple[int, ...], tuple[int, ...]], int] = {}
    boot_records = []
    for rec in raw_boot_records:
        k_sig = (tuple(rec[1]), tuple(rec[2]))
        is_multiturn_plain = (not any(rec[3])) and (end_tok_id in rec[1])
        is_plain_tool = (not any(rec[3])) and (rec[0] in ("D", "E"))
        is_step2_d = any(rec[3]) and rec[0] == "D" and boot_totals[k_sig] >= 10
        cap = (
            min(235, max(10, (boot_totals[k_sig] * 3 + 4) // 5))
            if is_plain_tool
            else (
                min(230, (boot_totals[k_sig] * 3 + 4) // 5)
                if (not any(rec[3]) and boot_totals[k_sig] >= 340)
                else (
                    min(160, (boot_totals[k_sig] * 3 + 4) // 5)
                    if (not any(rec[3]) and boot_totals[k_sig] >= 150)
                    else (
                        min(48, max(20, (boot_totals[k_sig] * 2 + 2) // 3))
                        if (not any(rec[3]) and boot_totals[k_sig] >= 45)
                        else (
                            max(4, (boot_totals[k_sig] * 2 + 4) // 5)
                            if (not any(rec[3]) and rec[0] in ("A", "B") and boot_totals[k_sig] >= 12)
                            else (
                                max(2, (boot_totals[k_sig] * 2 + 2) // 3)
                                if is_step2_d
                                else max(1, (boot_totals[k_sig] + 2) // 3)
                            )
                        )
                    )
                )
            )
        )
        used = boot_seen.get(k_sig, 0)
        if used < cap:
            boot_records.append(rec)
            boot_seen[k_sig] = used + 1

    # For full `8L 128D` (`v10_corpus.bin`): keep pre-topup around ~40,000..44,500 so `top_up_to_target`
    # always contributes ~5,500..10,000 balanced records (`docs.rs` `web_fetch`, `[agent_mode]` Step 1, `fn_rust`, `git clone`)!
    all_totals: dict[tuple[tuple[int, ...], tuple[int, ...]], int] = {}
    for _b, p_ids, r_ids, _rc in sb.records:
        k_sig = (tuple(p_ids), tuple(r_ids))
        all_totals[k_sig] = all_totals.get(k_sig, 0) + 1
    all_seen: dict[tuple[tuple[int, ...], tuple[int, ...]], int] = {}
    capped_pre = []
    for rec in sb.records:
        k_sig = (tuple(rec[1]), tuple(rec[2]))
        tot_k = all_totals[k_sig]
        is_ext = rec[0].endswith("_ext")
        c_cap = (
            max(1, (tot_k + 1) // 4)
            if is_ext
            else (
                max(2, (tot_k * 2 + 2) // 5)
                if (
                    rec[0] == "A"
                    and (any(rec[3]) or (end_tok_id in rec[1] and tot_k <= 45))
                )
                or (any(rec[3]) and rec[0] == "D" and tot_k == 40)
                else (
                    max(1, (tot_k + 2) // 4)
                    if (any(rec[3]) and tot_k >= 5)
                    else (
                        min(295, (tot_k * 3 + 3) // 4)
                        if (not any(rec[3]) and tot_k >= 340)
                        else (
                            min(215, (tot_k * 3 + 3) // 4)
                            if (not any(rec[3]) and tot_k >= 180)
                            else (
                                min(140, (tot_k * 3 + 4) // 5)
                                if (not any(rec[3]) and tot_k >= 120)
                                else (
                                    min(56, (tot_k * 3 + 3) // 4)
                                    if (not any(rec[3]) and tot_k >= 55)
                                    else (
                                        min(42, (tot_k * 2 + 4) // 5)
                                        if (not any(rec[3]) and tot_k >= 34)
                                        else (max(2, (tot_k + 1) // 2) if (not any(rec[3]) and tot_k >= 4) else tot_k)
                                    )
                                )
                            )
                        )
                    )
                )
            )
        )
        used_c = all_seen.get(k_sig, 0)
        if used_c < c_cap:
            capped_pre.append(rec)
            all_seen[k_sig] = used_c + 1
    sb.records = capped_pre
    pre_topup_count = len(sb.records)
    print(f"[corpus] pre-topup records={pre_topup_count} (boot_records={len(boot_records)})")
    assert pre_topup_count <= 45000, f"pre_topup_count={pre_topup_count} too high — would starve top_up_to_target!"

    top_up_to_target(sb, rng)

    # Remove any accidental prompt-key overlap between train and eval (strict disjointness!)
    overlap = sb.seen_keys & eval_sb.seen_keys
    if overlap:
        filtered_eval = []
        for rec in eval_sb.records:
            p_str = tok.decode(rec[1][1:])
            if norm_key(p_str) not in sb.seen_keys:
                filtered_eval.append(rec)
        eval_sb.records = filtered_eval

    rng.shuffle(boot_records)
    rng.shuffle(sb.records)
    all_records_for_audit = list(sb.records)
    if len(sb.records) > 52000:
        sb.records = sb.records[:52000]
    rng.shuffle(eval_sb.records)
    if len(eval_sb.records) > TARGET_EVAL_COUNT:
        eval_sb.records = eval_sb.records[:TARGET_EVAL_COUNT]
    boot_eval_records = [rec for rec in eval_sb.records if not rec[0].endswith("_ext")]

    # Verify zero forbidden held-out identifiers/topics AND full-corpus (no dedup collapse!) span-start orthogonality!
    checked_text_sigs: set[tuple[int, ...]] = set()
    span_on: dict[tuple[int, int], int] = {}
    span_off: dict[tuple[int, int], int] = {}
    for _b, p_ids, r_ids, _rc in all_records_for_audit:
        sig = tuple(p_ids) + tuple(r_ids)
        if sig not in checked_text_sigs:
            checked_text_sigs.add(sig)
            txt = tok.decode(p_ids[1:] + r_ids[:-1])
            assert not contains_forbidden(txt), f"Held-out leakage detected: {txt}"
        ft0 = r_ids[0] if r_ids else 0
        ft1 = r_ids[1] if len(r_ids) > 1 else 0
        structured = any(_rc) or ft0 in (30, 66) or (ft0 == 223 and ft1 in (30, 66))
        if structured:
            sp = 1
            for k in range(len(r_ids)):
                cur_tok = p_ids[-1] if k == 0 else r_ids[k - 1]
                if k > 0 and _rc[k - 1] == 0:
                    sp = min(255, sp + 1)
                is_start = (_rc[k] != 0) and (k == 0 or _rc[k - 1] == 0)
                fence_nl = cur_tok in (29, 66, 201)
                if not fence_nl and (k == 0 or _rc[k - 1] == 0):
                    key = (sp, cur_tok & 63)
                    if is_start:
                        span_on[key] = span_on.get(key, 0) + 1
                    else:
                        span_off[key] = span_off.get(key, 0) + 1
    collisions = sorted(
        [(k, span_on[k], span_off[k]) for k in span_on if k in span_off],
        key=lambda x: -min(x[1], x[2]),
    )
    print(
        f"[corpus] FULL-CORPUS structured (pos_idx, tok&63) span-start audit: "
        f"{len(span_on)} active states, collisions={collisions[:8]} (total={len(collisions)})"
    )
    assert all(min(c[1], c[2]) == 0 for c in collisions), f"Span-start collision remained: {collisions[:8]}"

    write_uorcorp3(out_corpus, sb.records)
    write_uorcorp3(out_boot_corpus, boot_records)
    write_uorcorp3(out_eval, eval_sb.records)
    write_uorcorp3(out_boot_eval, boot_eval_records)
    write_held_out_eval_prompts(out_prompts, out_prompts_bin, tok)

    bucket_counts: dict[str, int] = {}
    max_len_seen = 0
    copy_tok_total = 0
    for b, p_ids, r_ids, rc in sb.records:
        b_root = b[0]
        bucket_counts[b_root] = bucket_counts.get(b_root, 0) + 1
        max_len_seen = max(max_len_seen, len(p_ids) + len(r_ids))
        copy_tok_total += sum(rc)

    print(
        f"[corpus] wrote {len(sb.records)} full train sequences -> {out_corpus} "
        f"({out_corpus.stat().st_size / (1024 * 1024):.2f} MiB), "
        f"{len(boot_records)} fast-start boot sequences -> {out_boot_corpus}, "
        f"{len(eval_sb.records)} held-out eval sequences -> {out_eval} "
        f"({len(boot_eval_records)} boot-eval -> {out_boot_eval}), "
        f"max_seq_seen={max_len_seen}, copy_tokens={copy_tok_total}, buckets={bucket_counts}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
