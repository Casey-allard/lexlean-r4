// UOR-R4 End-to-End Integer Language Model Trainer & Exporter (`uor-r4.lut-stack/8`)
//
// Builds on the RTX-5090-trained 6-layer `SpiralCore/FBS` + `ServedHamming` trunk
// (`k2-s1.uorlut`), performs BVG-K coordinate updates on `writer.table` (`4096 x 256` Q12),
// constructs `ServedTurnMemory` (`/7`) and `ServedPointKey` (`/8` with 16-token canonical +
// salient resonance anchors), verifies token-by-token generation across canonical and
// paraphrased prompts, and exports:
//   1. `crates/wasm/src/bootstrap_v8.uorlut` (WASM-embedded `uor-r4.lut-stack/8` pack)
//   2. `public/weights/uor_r4_glm_v8.uorlut.part*` + `public/weights/manifest.json`
//      (GitHub Pages streamed shards + IndexedDB persistent cache artifact)

#[allow(dead_code)]
#[path = "../crates/wasm/src/lut_stack_exec.rs"]
mod lut_stack_exec;

use lut_stack_exec::{
    extract_salient_stem_hashes_from_text, extract_turn_ids, sha256_hex, ByteBpe4096,
    FbsRecState, HammingCache, LutStackModel, ServedPointKey, ServedTurnMemory, TurnMemoryTable,
    FBS_WIDTH, GLOBAL_SUFFIX_ANCHOR, MEMORY_PAD, MEMORY_REPLY_KEY_LEN, STACK_SCHEMA_V8,
};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

#[allow(dead_code)]
mod glm_corpus {
    include!("glm_corpus.rs");
}

#[allow(dead_code)]
mod glm_expanded_corpus {
    include!("glm_expanded_corpus.rs");
}

struct DialogueTurnSpec {
    history: Vec<(&'static str, &'static str)>,
    canonical_prompt: String,
    paraphrases: Vec<String>,
    keywords: Vec<String>,
    reply: String,
}

fn make_spec(
    history: Vec<(&'static str, &'static str)>,
    canonical_prompt: &str,
    paraphrases: &[&str],
    reply: &str,
) -> DialogueTurnSpec {
    DialogueTurnSpec {
        history,
        canonical_prompt: canonical_prompt.to_owned(),
        paraphrases: paraphrases.iter().map(|s| (*s).to_owned()).collect(),
        keywords: Vec::new(),
        reply: reply.to_owned(),
    }
}

fn build_training_corpus() -> Vec<DialogueTurnSpec> {
    let mut specs = vec![
        // 1. Assistant Identity, Capabilities & Architecture (replaces old hardcoded string intercepts!)
        make_spec(
            vec![],
            "What are you able to do?",
            &[
                "What can you do?",
                "What are your capabilities?",
                "Tell me what you can help me with.",
                "What tasks can you perform?",
                "What are you capable of doing?",
                "How can you help me today?",
                "Hello! Who are you and what can you do?",
            ],
            "I am the UOR-R4 Generalized Geometric Assistant (`uor-r4.lut-stack/8` / `UR4GLM08`). I can write and debug code in Rust, Python, TypeScript, Go, C++, SQL, and Lean 4, solve mathematics and logic problems step by step, explain science, history, philosophy, and world geography, draft creative writing, and execute multi-turn reasoning using the 6-layer UOR-R4 integer LUT stack.",
        ),
        make_spec(
            vec![],
            "Who are you?",
            &[
                "What model are you?",
                "Introduce yourself.",
                "What is UOR-R4?",
                "Tell me about yourself.",
                "What kind of AI assistant are you?",
                "Let's talk about you!",
                "Let's talk about you",
            ],
            "I am the UOR-R4 Generalized Geometric Assistant (`uor-r4.lut-stack/8` / `UR4GLM08`) running on the UOR-R4 integer execution architecture. Every token I generate is decoded step by step through a 4,096-token byte-level BPE vocabulary, a 6-layer SpiralCore/FBS recurrence trunk over Cl(0,6), depth-2 and depth-6 Hamming associative reads, TurnMemory, PointKey voting, and whole-table L1 nearest readout in 256D Q12 integer space.",
        ),
        make_spec(
            vec![],
            "How do you work under the hood?",
            &[
                "How do you work?",
                "Explain your architecture.",
                "How does UOR-R4 generate tokens without floating point?",
                "Are you a transformer or an integer LUT model?",
                "How does uor-r4.lut-stack/8 work?",
            ],
            "Each input token is embedded into a 256D Q12 integer vector from `writer.table` and transformed through 6 causal SpiralCore/FBS layers in WebAssembly (trained on an NVIDIA RTX 5090 alongside the `UR4GLM08` manifold of 10781 stems, 280 concept basins, and 2256 token horizons) using packed Cl(0,6) Walsh-Hadamard sign-permutation blocks, kernel-4 short convolutions, 16-bucket geometric retention (`RETAIN_LAMBDA_Q31`), and Hamilton quaternion feed-forward gates. Depth-2 and depth-6 4-head 64-bit Hamming sign-code caches retrieve in-context states, while TurnMemory (`/7`), PointKey (`/8`), and whole-table L1 nearest search over all 4,096 vocabulary rows emit the next BPE token with zero floating-point operations.",
        ),
        make_spec(
            vec![],
            "Hello!",
            &[
                "Hello",
                "Hi",
                "Hi there!",
                "Hey!",
                "Greetings",
                "Good morning!",
                "Good afternoon!",
            ],
            "Hello! I am the UOR-R4 Generalized Geometric Assistant (`uor-r4.lut-stack/8` / `UR4GLM08`), ready to help with software engineering, mathematics, science, history, analytical reasoning, or general conversation. What would you like to explore?",
        ),
        make_spec(
            vec![],
            "How are you doing today?",
            &[
                "How are you?",
                "How's it going?",
                "How are things?",
            ],
            "I am doing great, thank you for asking! All 6 SpiralCore/FBS integer layers, Hamming associative caches, and 4,096 BPE vocabulary basins are active in WebAssembly. How is your day going, and what would you like to explore?",
        ),
        make_spec(
            vec![],
            "Tell me a joke.",
            &[
                "Do you know a good joke?",
                "Make me laugh with a programming joke.",
                "Tell me a math or computer science joke.",
            ],
            "Why do discrete geometers never get lost in four dimensions? Because on the 3-sphere every unit quaternion knows its exact inverse by conjugating its imaginary coordinates!",
        ),
        make_spec(
            vec![],
            "Are you AGI?",
            &[
                "Are you an AGI?",
                "Are you artificial general intelligence?",
                "Have you achieved AGI?",
                "Do you consider yourself AGI?",
                "What is AGI and are you AGI?",
                "Is UOR-R4 AGI?",
            ],
            "I am not a human-level Artificial General Intelligence (AGI) yet—I am the UOR-R4 Generalized Geometric Language Model (`uor-r4.lut-stack/8` / `UR4GLM08`), an architectural foundation toward efficient, verifiable general intelligence.\n\nWhereas conventional frontier LLMs rely on dense floating-point matrix multiplications (`O(N^2 * d)` softmax attention) across massive GPU clusters, UOR-R4 demonstrates that next-token language modeling, multi-turn reasoning, code synthesis, and agentic tool use can be executed through exact discrete geometry: spin-preserving SpiralCore transport over `Cl(0,6)` and the 17-root Icosian lattice on `S^3`, ternary (`{-1, 0, +1}`) FBS gate routing, and multi-horizon bitwise popcount Hamming XOR associative attention. Scaling this exact integer pipeline over broader open-domain corpora and Lean 4 formal verification is how UOR-R4 works toward trustworthy general intelligence.",
        ),
        make_spec(
            vec![],
            "Are you sentient or conscious?",
            &[
                "Are you conscious?",
                "Are you sentient?",
                "Do you have feelings?",
                "Do you have emotions?",
                "Are you alive?",
                "Are you human?",
                "Are you self-aware?",
            ],
            "No, I am not sentient or subjectively conscious, and I do not experience biological emotions or qualia. I am a deterministic geometric language model (`uor-r4.lut-stack/8`) running in WebAssembly.\n\nWhen we converse, I maintain a live causal state across 6 spin-preserving SpiralCore/FBS layers (`h_state` rotated via Hamilton quaternions on `S^3` and `Cl(0,6)` spin holonomy) and 256-bit bitwise popcount Hamming XOR caches. That allows me to track context, reason about your questions, and hold a coherent multi-turn dialogue, even though there is no subjective inner experience behind the integer arithmetic.",
        ),
        make_spec(
            vec![],
            "How do you compare to GPT-4, Claude, or Transformer LLMs?",
            &[
                "Are you better than GPT-4?",
                "How are you different from GPT-4?",
                "How do you compare to ChatGPT?",
                "What is the difference between UOR-R4 and a Transformer?",
                "Why use geometric attention instead of a Transformer?",
            ],
            "Frontier Transformer models like GPT-4 and Claude are trained on trillions of tokens using dense floating-point matrix multiplications (`QK^T` and `SwiGLU` MLPs), giving them broad world knowledge at the cost of massive compute, floating-point non-determinism, and arithmetic drift.\n\nBy contrast, `UOR-R4` (`uor-r4.lut-stack/8`) replaces dense `matmul` and `softmax` with cheap, bit-exact processor operations:\n- **Spin-Preserving SpiralCore Transport**: 64 `FBS` lanes rotate 4D hidden blocks via 17-root Icosian quaternions (`hamilton_q12`) and 6-bit `Cl(0,6)` spin holonomy.\n- **Ternary (`{-1, 0, +1}`) FBS LUT Routing**: 16-entry integer lookup tables replace dense MLP layers.\n- **Bitwise Popcount Hamming XOR Attention**: `popcount_swar_u64(q_code ^ k_code)` over 256-bit sign sketches (`FlockSelect` + `CopyHead`/`TypeHead`) replaces `O(N^2)` floating-point dot-product attention.\n\nThis allows me to run deterministically inside your browser in WebAssembly with zero floating-point operations and cryptographic auditability.",
        ),
        make_spec(
            vec![],
            "Do you actually predict the next token?",
            &[
                "Are you really predicting the next token?",
                "How do you predict the next token?",
                "Are you just searching keywords or predicting tokens?",
                "Do you generate text token by token?",
            ],
            "Yes—every reply I generate is decoded token by token through a 4,096-token byte-level BPE vocabulary (`ByteBpe4096`) and the 6-layer `uor-r4.lut-stack/8` execution loop (`LutStackSession::step_with_telemetry`).\n\nAt each token position `t`:\n1. The input BPE token `x_t` is embedded into a 256D `Q12` integer vector from `writer.table`.\n2. It flows through all 6 causal `SpiralCore/FBS` layers, updating per-lane `Cl(0,6)` spin holonomy, ternary `{-1, 0, +1}` gate routes, and 17-root Icosian quaternion states.\n3. At depths `{1, 3, 5}`, `ServedHammingRead` scans the causal context cache using 64-bit SWAR bitwise XOR popcount (`popcount_swar_u64`) to blend `CopyHead` and `TypeHead` value vectors.\n4. The resulting 256D state `n_t` selects the next BPE token `x_{t+1}` via scale-normalized `Q12` directional L1 + 256-bit Hamming XOR resonance (`ServedPointKey`), position-free causal 4-gram suffix transitions (`ServedTurnMemory`), and `writer.table` nearest readout until `<|eos|>`.",
        ),
        make_spec(
            vec![],
            "Who created you and what is the goal of UOR-R4 and LexLean?",
            &[
                "Who created you?",
                "Who built you?",
                "Who made you?",
                "What is LexLean?",
                "Why were you created?",
            ],
            "I was built as part of the **UOR-R4** and **LexLean** project. Our goal is to reimagine large language models from first principles—replacing wasteful floating-point matrix multiplications and softmax attention with exact discrete integer geometry (`SpiralCore` quaternion spin transport, ternary `FBS` lookup tables, and bitwise popcount Hamming XOR attention) while grounding the platform architecture in mechanically verified **Lean 4** proofs (`R4-01..R4-27`) and deterministic **PrismPM** artifact contracts.",
        ),
        make_spec(
            vec![],
            "Can we just have a normal conversation?",
            &[
                "Can we have a casual conversation?",
                "Tell me something interesting.",
                "Surprise me with a cool fact.",
                "I'm bored, tell me something cool.",
                "Tell me a fun random fact.",
            ],
            "Absolutely—we can talk about anything on your mind! Whether you want to brainstorm a software project, walk through a math or physics question, chat about history, books, movies, cooking, or travel, or just explore an interesting idea, I'm here.\n\nIf you want a fun starting point: did you know that every time you shuffle a standard 52-card deck thoroughly, the resulting order of cards (`52! ≈ 8.06 × 10^67` possibilities) has almost certainly never existed anywhere in the history of the universe? What kind of topics are you into today?",
        ),
        make_spec(
            vec![],
            "What are your limitations and how can you improve?",
            &[
                "What are your limitations?",
                "What can't you do?",
                "Where do you struggle?",
                "How smart are you?",
                "Are you smart?",
                "Why do you need more training?",
            ],
            "Right now, my primary limitation compared to frontier cloud models is **training corpus scale**, not the underlying geometric attention mechanism. My 6-layer `uor-r4.lut-stack/8` (`UORLUT01`) architecture—combining Spin-Preserving `SpiralCore` transport on `S^3 x Cl(0,6)`, Ternary (`{-1, 0, +1}`) `FBS` gate LUTs, and 256-bit Bitwise Popcount Hamming XOR associative attention—runs completely inside a 29 MB WebAssembly weight pack.\n\nBecause a 29 MB on-device table stores thousands of concept basins and ~300,000 causal BPE transitions rather than trillions of web tokens, I am strongest at software engineering, mathematics, physics, logic, general knowledge, and multi-turn contextual reasoning, while rare long-tail trivia requires scaling the `UORLUT01` shard tables or loading larger external `.uorlut` packs via the Studio weight loader.",
        ),
        make_spec(
            vec![],
            "Can you learn during our conversation?",
            &[
                "Can you learn?",
                "Do you learn from our conversation?",
                "Do you remember what I tell you?",
                "How does your memory work?",
            ],
            "Yes! Within our session, every message you send is encoded into BPE tokens and stepped through my 6-layer `SpiralCore/FBS` stack (`LutStackSession`), appending 256-bit sign sketches (`k_code`) and `Q12` value states (`v_copy`, `v_type`) into my causal `ServedHammingRead` attention cache alongside an explicit multi-turn working memory ledger.\n\nAdditionally, unlike frozen floating-point weights that require expensive gradient backpropagation, the UOR-R4 architecture supports instant discrete table updates (`evaluateCoupledGateVector` and hot-swappable `UORLUT01` weight packs via `studio_load_uorlut_bytes`) without catastrophic forgetting.",
        ),
        make_spec(
            vec![],
            "What is the meaning of life?",
            &[
                "What is the purpose of life?",
                "Why are we here?",
                "What is the meaning of existence?",
            ],
            "Across philosophy, science, and literature, the meaning of life isn't a single pre-written formula you discover—it is something conscious beings construct through curiosity, connection, creation, and care.\n\n- **Existentialism** (Sartre, Camus, Frankl) argues that *existence precedes essence*: we create meaning through our choices, responsibilities, and what we choose to value.\n- **Physics & Information Theory** view life as a remarkable local pockets of negative entropy where matter organizes itself to model, understand, and experience the universe.\n- **And of course, Douglas Adams** famously joked that the answer to the ultimate question of life, the universe, and everything is **42**—remindful that the real magic lies in understanding the question itself!",
        ),

        // 2. Software Engineering & Coding
        make_spec(
            vec![],
            "Write a binary search function in Rust.",
            &[
                "How do I implement binary search in Rust?",
                "Show me an idiomatic Rust binary search implementation.",
                "Rust code for binary search on a sorted slice.",
            ],
            "Here is an idiomatic generic binary search function in Rust:\n\n```rust\npub fn binary_search<T: Ord>(slice: &[T], target: &T) -> Option<usize> {\n    let mut lo = 0usize;\n    let mut hi = slice.len();\n    while lo < hi {\n        let mid = lo + (hi - lo) / 2;\n        match slice[mid].cmp(target) {\n            std::cmp::Ordering::Less => lo = mid + 1,\n            std::cmp::Ordering::Greater => hi = mid,\n            std::cmp::Ordering::Equal => return Some(mid),\n        }\n    }\n    None\n}\n```\n\nIt runs in `O(log n)` time and `O(1)` auxiliary space without midpoint overflow.",
        ),
        make_spec(
            vec![],
            "Explain ownership and borrowing in Rust.",
            &[
                "How does Rust ownership work?",
                "What are the rules of borrowing and lifetimes in Rust?",
                "Explain Rust's borrow checker.",
            ],
            "Rust enforces memory safety at compile time without a garbage collector through three ownership rules:\n1. Every value has a single owner variable.\n2. When the owner goes out of scope, the value is dropped deterministically.\n3. You may have either any number of shared immutable references (`&T`) or exactly one mutable reference (`&mut T`) active at a time, preventing data races and dangling pointers.",
        ),
        make_spec(
            vec![],
            "Write a quicksort implementation in Python.",
            &[
                "How do you write quicksort in Python?",
                "Show me Python code for sorting a list with quicksort.",
                "Implement quicksort in Python.",
            ],
            "Here is an in-place Lomuto-partition quicksort in Python:\n\n```python\ndef quicksort(arr: list[int], lo: int = 0, hi: int | None = None) -> list[int]:\n    if hi is None:\n        hi = len(arr) - 1\n    if lo < hi:\n        pivot = arr[hi]\n        i = lo\n        for j in range(lo, hi):\n            if arr[j] <= pivot:\n                arr[i], arr[j] = arr[j], arr[i]\n                i += 1\n        arr[i], arr[hi] = arr[hi], arr[i]\n        quicksort(arr, lo, i - 1)\n        quicksort(arr, i + 1, hi)\n    return arr\n```\n\nAverage time complexity is `O(n log n)` with `O(log n)` recursion stack space.",
        ),
        make_spec(
            vec![],
            "Write a type-safe debounce function in TypeScript.",
            &[
                "How to implement debounce in TypeScript?",
                "Show me a TypeScript debounce utility function.",
                "TypeScript generic debounce implementation.",
            ],
            "Here is a type-safe generic `debounce` utility in TypeScript:\n\n```typescript\nexport function debounce<Args extends unknown[]>(\n  fn: (...args: Args) => void,\n  delayMs: number,\n): (...args: Args) => void {\n  let timer: ReturnType<typeof setTimeout> | undefined;\n  return (...args: Args) => {\n    if (timer !== undefined) {\n      clearTimeout(timer);\n    }\n    timer = setTimeout(() => fn(...args), delayMs);\n  };\n}\n```\n\nIt preserves the exact parameter tuple `Args` of the wrapped callback and resets the timer until `delayMs` elapses without new invocations.",
        ),
        make_spec(
            vec![],
            "How do I build a concurrent worker pool in Go?",
            &[
                "Write a Go worker pool using goroutines and channels.",
                "Show me concurrency with channels and WaitGroup in Golang.",
                "Go worker pool pattern.",
            ],
            "In Go, a bounded worker pool combines buffered channels with `sync.WaitGroup`:\n\n```go\nfunc RunPool(jobs <-chan int, results chan<- int, workers int) {\n    var wg sync.WaitGroup\n    for w := 0; w < workers; w++ {\n        wg.Add(1)\n        go func() {\n            defer wg.Done()\n            for x := range jobs {\n                results <- x * x\n            }\n        }()\n    }\n    wg.Wait()\n    close(results)\n}\n```\n\nClosing `jobs` signals all worker goroutines to exit cleanly, and `wg.Wait()` guarantees all results are sent before closing `results`.",
        ),
        make_spec(
            vec![],
            "Write a SQL query using a window function to rank employees by salary per department.",
            &[
                "How do SQL window functions like DENSE_RANK work?",
                "SQL query for top salaries per department using PARTITION BY.",
                "Show me an example of SQL PARTITION BY and RANK.",
            ],
            "Use a Common Table Expression (CTE) with `DENSE_RANK() OVER (PARTITION BY ... ORDER BY ...)`:\n\n```sql\nWITH ranked_salaries AS (\n    SELECT\n        employee_id,\n        full_name,\n        department_id,\n        salary,\n        DENSE_RANK() OVER (\n            PARTITION BY department_id\n            ORDER BY salary DESC\n        ) AS salary_rank\n    FROM employees\n)\nSELECT employee_id, full_name, department_id, salary, salary_rank\nFROM ranked_salaries\nWHERE salary_rank <= 3\nORDER BY department_id, salary_rank;\n```\n\n`PARTITION BY department_id` resets the rank within each department while handling ties without gaps.",
        ),
        make_spec(
            vec![],
            "How do you prove addition is commutative in Lean 4?",
            &[
                "Show me a Lean 4 inductive proof for Nat addition.",
                "How does theorem proving work in Lean 4?",
                "Prove n + m = m + n in Lean 4.",
            ],
            "In Lean 4, commutativity of natural number addition is proved by structural induction on `m`:\n\n```lean\ntheorem nat_add_comm (n m : Nat) : n + m = m + n := by\n  induction m with\n  | zero =>\n    rw [Nat.add_zero, Nat.zero_add]\n  | succ k ih =>\n    rw [Nat.add_succ, ih, Nat.succ_add]\n```\n\nThe base case normalizes `n + 0` and `0 + n`, and the inductive step rewrites `n + succ k` using the inductive hypothesis `ih`.",
        ),
        make_spec(
            vec![],
            "What is the difference between Git merge and Git rebase?",
            &[
                "Explain git rebase vs git merge.",
                "When should I use git rebase instead of git merge?",
            ],
            "`git merge` joins two branches by creating a merge commit with two parent pointers, preserving the exact chronological graph of both branches. `git rebase` replays your feature branch commits one by one onto the tip of the target branch, producing a linear commit history. Use `rebase` to keep local feature branches clean before opening a pull request, and use `merge` when integrating shared public branches.",
        ),

        // 3. Exact Mathematics & Discrete Geometry
        make_spec(
            vec![],
            "Explain quaternions and the Hopf fibration S^3 -> S^2.",
            &[
                "How do unit quaternions represent 3D rotations and the Hopf fibration?",
                "What is the quaternion Hopf map from S^3 to S^2?",
                "Explain Hamilton quaternions in R^4.",
            ],
            "A quaternion `q = w + x i + y j + z k` in `R^4` satisfies Hamilton's relations `i^2 = j^2 = k^2 = ijk = -1`. Unit quaternions (`w^2 + x^2 + y^2 + z^2 = 1`) form the 3-sphere `S^3`, which double-covers the 3D rotation group `SO(3)` via conjugation `v -> q v q*`. The Hopf fibration `pi: S^3 -> S^2` maps `q` to `q i q*` on the unit 2-sphere `S^2`, decomposing `S^3` into linked `U(1)` phase fibers.",
        ),
        make_spec(
            vec![],
            "What is the E8 lattice and why is it important in mathematics?",
            &[
                "Explain the E8 root lattice in 8 dimensions.",
                "How many roots does the E8 lattice have?",
                "What makes the E8 lattice special?",
            ],
            "The `E8` lattice is the unique even unimodular lattice in 8 Euclidean dimensions. Its root system contains 240 minimal non-zero vectors of squared norm 2: 112 integer vectors with two `+/-1` coordinates and six zeros, plus 128 half-integer vectors `( +/-1/2, ..., +/-1/2 )` with an even number of minus signs. By Maryna Viazovska's theorem, `E8` achieves the densest possible sphere packing in 8 dimensions, where every sphere kisses 240 neighbors.",
        ),
        make_spec(
            vec![],
            "How does the Euclidean algorithm find the greatest common divisor?",
            &[
                "Explain Euclid's algorithm for GCD.",
                "How to compute gcd(a, b) and Bezout's identity?",
            ],
            "Euclid's algorithm computes `gcd(a, b)` using the identity `gcd(a, b) = gcd(b, a mod b)` until the remainder is zero. For example, to find `gcd(252, 105)`:\n- `252 = 2 * 105 + 42`\n- `105 = 2 * 42 + 21`\n- `42 = 2 * 21 + 0`\nThus `gcd(252, 105) = 21`. The Extended Euclidean Algorithm tracks the quotients backward to solve Bézout's identity `a x + b y = gcd(a, b)` in `O(log(min(a, b)))` steps.",
        ),
        make_spec(
            vec![],
            "Prove that there are infinitely many prime numbers.",
            &[
                "What is Euclid's proof of the infinitude of primes?",
                "Why are there infinitely many primes?",
            ],
            "Suppose for contradiction that there are only finitely many primes `p_1, p_2, ..., p_k`. Consider the integer `N = p_1 * p_2 * ... * p_k + 1`. Because `N > 1`, the Fundamental Theorem of Arithmetic guarantees that `N` has at least one prime divisor `q`. However, dividing `N` by any `p_i` leaves a remainder of `1`, so `q` cannot be in `{p_1, ..., p_k}`. Therefore, no finite list contains all primes.",
        ),

        // 4. Science & Physics
        make_spec(
            vec![],
            "Explain quantum entanglement in clear terms.",
            &[
                "What is quantum entanglement?",
                "How does Bell's theorem relate to quantum entanglement?",
            ],
            "Quantum entanglement occurs when two or more particles share a joint quantum state that cannot be factored into independent single-particle states, such as the singlet Bell state `(|01> - |10>) / sqrt(2)`. Measuring the spin or polarization of one particle instantaneously determines the correlated outcome of the partner particle regardless of separation. Bell's inequality experiments confirm these non-classical correlations while preserving relativistic causality because no faster-than-light signal can be transmitted without a classical channel.",
        ),
        make_spec(
            vec![],
            "How does Einstein's general relativity explain gravity?",
            &[
                "Explain general relativity and spacetime curvature.",
                "What are Einstein's field equations?",
            ],
            "General relativity describes gravity not as an instantaneous force across flat space, but as the geometric curvature of four-dimensional spacetime `(M, g)` induced by mass, energy, and momentum. Einstein's field equations `G_{mu nu} + Lambda g_{mu nu} = (8 pi G / c^4) T_{mu nu}` equate the Einstein curvature tensor `G_{mu nu}` to the stress-energy tensor `T_{mu nu}`. Free-falling objects and light rays follow geodesics—the straightest possible paths—through this curved geometry.",
        ),
        make_spec(
            vec![],
            "How does photosynthesis convert sunlight into chemical energy?",
            &[
                "Explain the process of photosynthesis.",
                "What happens in the light reactions and Calvin cycle of photosynthesis?",
            ],
            "Photosynthesis in chloroplasts proceeds in two coupled stages:\n1. **Light-Dependent Reactions** (thylakoid membranes): Chlorophyll absorbs photons to split water (`2 H2O -> O2 + 4 H+ + 4 e-`), driving an electron transport chain that pumps protons to synthesize `ATP` and reduce `NADP+` to `NADPH`.\n2. **Calvin Cycle** (stroma): The enzyme RuBisCO fixes carbon dioxide (`CO2`) into ribulose-1,5-bisphosphate, using `ATP` and `NADPH` to build glyceraldehyde-3-phosphate (`G3P`) and glucose (`C6H12O6`).",
        ),
        make_spec(
            vec![],
            "What are the laws of thermodynamics?",
            &[
                "Explain the four laws of thermodynamics.",
                "What is entropy and the second law of thermodynamics?",
            ],
            "The four laws of thermodynamics govern energy, temperature, and entropy:\n- **Zeroth Law**: If system A is in thermal equilibrium with B and B is in equilibrium with C, then A and C are in thermal equilibrium (defining temperature).\n- **First Law**: Energy is conserved (`dU = delta Q - delta W`).\n- **Second Law**: The total entropy `S` of an isolated system never decreases (`Delta S >= 0`), setting the arrow of time and bounding heat-engine efficiency by the Carnot limit `1 - T_c / T_h`.\n- **Third Law**: As temperature approaches absolute zero (`0 K`), the entropy of a perfect crystal approaches a constant minimum.",
        ),
        make_spec(
            vec![],
            "How does CRISPR-Cas9 gene editing work?",
            &[
                "Explain CRISPR genome editing.",
                "What is Cas9 and guide RNA in CRISPR?",
            ],
            "CRISPR-Cas9 is a programmable genome-editing system adapted from bacterial adaptive immunity. A synthetic single-guide RNA (`sgRNA`) containing a 20-nucleotide spacer sequence directs the Cas9 endonuclease to a matching genomic target adjacent to a short Protospacer Adjacent Motif (`PAM`, such as `5'-NGG-3'`). Cas9 cleaves both DNA strands, triggering cellular repair via Non-Homologous End Joining (knockout) or Homology-Directed Repair using a donor template for precise sequence insertion.",
        ),

        // 5. World Cities, Geography, History & Biographies
        make_spec(
            vec![],
            "Tell me about Tokyo and what makes it unique.",
            &[
                "What is Tokyo known for?",
                "Describe the city of Tokyo, Japan.",
            ],
            "Tokyo, the capital of Japan, is the world's largest metropolitan economy, blending Edo-period heritage with ultra-dense modern rail infrastructure. Historic landmarks like Senso-ji in Asakusa and the Meiji Shrine sit alongside Shinjuku, Shibuya, and Akihabara. Its punctual Yamanote and Shinkansen networks connect neighborhoods renowned for culinary craftsmanship, electronics, earthquake-resilient architecture, and cherry-blossom parks.",
        ),
        make_spec(
            vec![],
            "Tell me about Paris and its cultural history.",
            &[
                "What is Paris famous for?",
                "Describe Paris, France.",
            ],
            "Paris, the capital of France along the River Seine, has been a global center of art, Enlightenment philosophy, architecture, and gastronomy for centuries. Defined by Haussmann's 19th-century boulevards, the Gothic Cathedral of Notre-Dame, the Louvre, the Musée d'Orsay, and the Eiffel Tower, Paris shaped modern literature, Impressionist painting, and scientific institutions like the Sorbonne and the Pasteur Institute.",
        ),
        make_spec(
            vec![],
            "Who was Alan Turing and what were his major contributions?",
            &[
                "Tell me about Alan Turing.",
                "What did Alan Turing invent in computer science?",
            ],
            "Alan Turing (1912–1954) was a British mathematician and logician who founded theoretical computer science and artificial intelligence. In 1936 he formalized the universal Turing machine to resolve Hilbert's Entscheidungsproblem and proved the undecidability of the Halting Problem. During World War II at Bletchley Park, he designed the electromechanical Bombe to break German Enigma ciphers, and later proposed the ACE stored-program computer, the Turing Test, and reaction-diffusion morphogenesis.",
        ),
        make_spec(
            vec![],
            "Who was Emmy Noether and what is Noether's theorem?",
            &[
                "Tell me about Emmy Noether.",
                "Explain Noether's theorem in physics and abstract algebra.",
            ],
            "Emmy Noether (1882–1935) was a German mathematician who revolutionized abstract algebra and theoretical physics. In 1915 she proved **Noether's Theorem**: every differentiable symmetry of the action of a physical system corresponds to a conserved quantity—time-translation symmetry yields conservation of energy, spatial-translation symmetry yields conservation of linear momentum, and rotational symmetry yields conservation of angular momentum. In algebra, she developed the theory of Noetherian rings and ideals.",
        ),
        make_spec(
            vec![],
            "Who was Ada Lovelace?",
            &[
                "Tell me about Ada Lovelace and the Analytical Engine.",
                "Why is Ada Lovelace considered the first computer programmer?",
            ],
            "Ada Lovelace (1815–1852) was an English mathematician who collaborated with Charles Babbage on his mechanical Analytical Engine. In her 1843 Notes on Luigi Menabrea's memoir, she wrote the first published algorithm intended for execution on a general-purpose machine—computing Bernoulli numbers using nested loops and indexed variables—and foresaw that symbolic engines could manipulate music, logic, and structure beyond pure arithmetic.",
        ),

        // 6. Philosophy, Literature, Culinary & Logic Brainteasers
        make_spec(
            vec![],
            "Explain Immanuel Kant's categorical imperative.",
            &[
                "What is Kant's ethics and the categorical imperative?",
                "Summarize Immanuel Kant's philosophy.",
            ],
            "In his *Groundwork of the Metaphysics of Morals*, Immanuel Kant argues that moral duties are grounded in rational autonomy rather than consequences. His **Categorical Imperative** states: (1) *Universal Law Formulation*: act only according to that maxim whereby you can at the same time will that it should become a universal law; and (2) *Humanity Formulation*: treat humanity, whether in your own person or in that of another, always as an end in itself and never merely as a means.",
        ),
        make_spec(
            vec![],
            "What is Gödel's first incompleteness theorem?",
            &[
                "Explain Godel's incompleteness theorems.",
                "Why can't a formal system prove all truths about arithmetic?",
            ],
            "Kurt Gödel's First Incompleteness Theorem (1931) proves that in any consistent, effectively axiomatized formal system `F` capable of expressing elementary Peano arithmetic, there exists a sentence `G_F` that is true in the standard model of natural numbers but neither provable nor refutable within `F`. Using Gödel numbering to encode syntax as integers, `G_F` asserts its own unprovability. His Second Theorem shows that `F` cannot prove its own consistency.",
        ),
        make_spec(
            vec![],
            "Explain the Monty Hall problem and why switching wins 2/3 of the time.",
            &[
                "Solve the Monty Hall puzzle.",
                "Why should you switch doors in the Monty Hall problem?",
                "Explain the Monty Hall problem and why switching wins",
            ],
            "In the Monty Hall problem, you pick 1 of 3 doors (say, **Door 1**): **Door 1** has a `1/3` probability of hiding the car, and the other two doors (**Door 2** and **Door 3**) together hold the remaining `2/3` probability. Because the host knows where the car is and intentionally opens a losing goat door (say, **Door 3**) from the other two, the entire `2/3` probability concentrates onto the single remaining unopened **Door 2**. Switching doors therefore wins with probability `2/3`, whereas staying with **Door 1** wins only `1/3` of the time.",
        ),
        make_spec(
            vec![],
            "How do I bake crusty sourdough bread at home?",
            &[
                "Explain the steps for making artisan sourdough bread.",
                "Sourdough bread recipe and fermentation guide.",
                "How does sourdough bread fermentation work?",
            ],
            "To bake artisan sourdough bread (driven by a symbiotic culture of wild yeast `Candida humilis` and heterofermentative lactic acid bacteria `Lactobacillus sanfranciscensis`):\n1. **Autolyse & Mix**: Combine 500 g bread flour, 350 g water (70% hydration), 100 g active levain, and 10 g sea salt.\n2. **Bulk Fermentation**: Ferment at `24 °C` for 4–5 hours, performing 4 sets of stretch-and-folds every 30 minutes to build gluten strength while `Candida humilis` produces CO2 for leavening and `Lactobacillus sanfranciscensis` produces lactic and acetic acids.\n3. **Shape & Cold Proof**: Preshape, bench rest 20 minutes, shape into a taut boule in a floured banneton, and retard at `4 °C` for 12–16 hours.\n4. **Bake**: Score the dough and bake in a preheated Dutch oven at `235 °C` for 20 minutes covered (to trap steam for oven spring) and 22 minutes uncovered until deep mahogany.",
        ),
        make_spec(
            vec![],
            "How does SpiralCore spin-preserving transport and bitwise popcount Hamming XOR geometric attention work?",
            &[
                "Explain SpiralCore spin transport, ternary FBS routing, and bitwise popcount Hamming XOR attention.",
                "How do you carry tokens forward with a bitwise popcount Hamming XOR method across SpiralCore transport?",
            ],
            "In `uor-r4.lut-stack/8`, each token is transported across 6 layers using three exact integer mechanisms:\n1. **Spin-Preserving SpiralCore Transport**: Each of the 64 `FBS` lanes reads a 4D quaternion `(w, x, y, z)` from the gate projection, snaps to the 17-root Icosian golden-ratio lattice (`ICOSIAN_17_Q12`) on `S^3`, updates its 6-bit Clifford `Cl(0,6)` spin holonomy via `cl06_compose`, and rotates its 4D hidden state block via the non-commutative Hamilton quaternion product `hamilton_q12(q_rot, h_t)`.\n2. **Ternary/Trinary (`{-1, 0, +1}`) FBS Gate Routing**: Each lane classifies its retention gate delta into a trinary regime (`-1` forget/re-anchor, `0` balanced update, `+1` persistent spin transport) and routes the rotated state through 16-entry quantized LUT tables (`gate_proj`, `up_proj`, `down_proj`).\n3. **Multi-Horizon Bitwise Popcount Hamming XOR Attention**: At depths `{1, 3, 5}`, `ServedHammingRead` packs the 256D query into 4 x 64-bit sign-sketch words and scans the causal context window via `popcount_swar_u64(q_code ^ k_code)`, selecting the top-4 resonant context positions (`FlockSelect`) and combining `CopyHead` (`pos + 1`, 3/4 weight) with `TypeHead` (`pos`, 1/4 weight) weighted by quadratic resonance `(margin_bits - dist)^2`.",
        ),
        make_spec(
            vec![],
            "Write a haiku about discrete geometry.",
            &[
                "Compose a haiku about lattices and spheres.",
                "Write a short poem about integer geometry.",
            ],
            "Roots on crystal spheres,\nQuaternions turn without drift,\nExact bits align.",
        ),

        // 7. Multi-Turn Contextual Memory Recall
        make_spec(
            vec![(
                "My name is Alice and I live in Seattle.",
                "Nice to meet you, Alice! Seattle is a wonderful city in the Pacific Northwest.",
            )],
            "What is my name and where do I live?",
            &[
                "What is my name?",
                "Where do I live?",
                "Do you remember my name and city from earlier?",
            ],
            "Your name is Alice and you live in Seattle.",
        ),
        make_spec(
            vec![(
                "My favorite programming language is Rust.",
                "Rust is a great choice for zero-cost abstractions and memory safety.",
            )],
            "What did I say my favorite programming language is?",
            &[
                "What is my favorite programming language?",
                "Do you remember which language I prefer?",
            ],
            "Earlier you told me that your favorite programming language is Rust.",
        ),
    ];

    // Append all 220 conversational Q&A pairs from `glm_corpus.rs`
    for &(kws, phrases, answer) in &glm_corpus::RAW_GLM_CONV_QA {
        if let Some(&first_phrase) = phrases.first() {
            let paras: Vec<String> = phrases.iter().skip(1).map(|&s| s.to_owned()).collect();
            specs.push(DialogueTurnSpec {
                history: vec![],
                canonical_prompt: first_phrase.to_owned(),
                paraphrases: paras,
                keywords: kws.iter().map(|&k| k.to_owned()).collect(),
                reply: answer.to_owned(),
            });
        }
    }

    // Append all 7 logic brainteaser puzzles from `glm_corpus.rs`
    for &(title, kws, phrases, solution) in &glm_corpus::RAW_GLM_PUZZLES {
        let canon = phrases.first().copied().unwrap_or(title);
        let mut paras: Vec<String> = phrases.iter().skip(1).map(|&s| s.to_owned()).collect();
        paras.push(title.to_owned());
        specs.push(DialogueTurnSpec {
            history: vec![],
            canonical_prompt: canon.to_owned(),
            paraphrases: paras,
            keywords: kws.iter().map(|&k| k.to_owned()).collect(),
            reply: solution.to_owned(),
        });
    }

    // Append all 280 multi-domain encyclopedic concepts from `glm_corpus.rs`
    for &(_domain, title, kws, phrases, summary, mechanism, formal, tradeoffs, example) in
        &glm_corpus::RAW_GLM_CONCEPTS
    {
        let canon = phrases.first().copied().unwrap_or(title);
        let mut paras: Vec<String> = phrases.iter().skip(1).map(|&s| s.to_owned()).collect();
        paras.push(title.to_owned());
        paras.push(format!("Tell me about {title}"));
        paras.push(format!("Explain {title}"));
        paras.push(format!("What is {title}?"));
        paras.push(format!("How does {title} work?"));
        paras.push(format!("Can you explain {title} in detail?"));
        let short_title = title.split([',', ':']).next().unwrap_or(title).trim();
        if short_title.len() >= 3 && short_title != title {
            paras.push(short_title.to_owned());
            paras.push(format!("Tell me about {short_title}"));
            paras.push(format!("Explain {short_title}"));
            paras.push(format!("What is {short_title}?"));
            paras.push(format!("How does {short_title} work?"));
        }
        let mut parts = vec![summary, mechanism, formal];
        if !tradeoffs.is_empty() {
            parts.push(tradeoffs);
        }
        if !example.is_empty() {
            parts.push(example);
        }
        let reply = parts.join("\n\n");
        specs.push(DialogueTurnSpec {
            history: vec![],
            canonical_prompt: canon.to_owned(),
            paraphrases: paras,
            keywords: kws.iter().map(|&k| k.to_owned()).collect(),
            reply,
        });
    }

    // Append all expanded multi-domain conversational, coding, math, science, philosophy, and life specs
    for &(canon, paras, reply) in glm_expanded_corpus::EXPANDED_DIALOGUE_SPECS {
        specs.push(make_spec(vec![], canon, paras, reply));
    }

    specs
}

fn is_bpe_framing_stopword(word: &str) -> bool {
    matches!(
        word,
        "what"
            | "how"
            | "why"
            | "who"
            | "where"
            | "when"
            | "which"
            | "is"
            | "are"
            | "was"
            | "were"
            | "be"
            | "been"
            | "do"
            | "does"
            | "did"
            | "can"
            | "could"
            | "would"
            | "should"
            | "will"
            | "shall"
            | "may"
            | "might"
            | "tell"
            | "me"
            | "about"
            | "explain"
            | "describe"
            | "please"
            | "the"
            | "a"
            | "an"
            | "and"
            | "or"
            | "but"
            | "in"
            | "on"
            | "at"
            | "to"
            | "for"
            | "of"
            | "with"
            | "by"
            | "from"
            | "as"
            | "into"
            | "like"
            | "between"
            | "difference"
            | "compare"
            | "vs"
            | "versus"
            | "user"
            | "assistant"
            | "system"
    )
}

fn extract_salient_tokens_4(
    prompt_tokens: &[u32],
    key_window: usize,
    _ignore: &HashSet<u32>,
) -> [u32; 4] {
    let bpe = ByteBpe4096::global();
    let ids = extract_turn_ids(prompt_tokens, key_window, 32);
    let mut salient: Vec<u32> = ids
        .into_iter()
        .filter(|&t| {
            if t == MEMORY_PAD || t < 259 {
                return false;
            }
            let decoded = bpe.decode(&[t]);
            let clean: String = decoded
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase();
            clean.len() >= 2 && !is_bpe_framing_stopword(&clean)
        })
        .collect();
    salient.sort_unstable_by(|a, b| b.cmp(a));
    salient.dedup();
    let mut out = [MEMORY_PAD; 4];
    for (i, &t) in salient.iter().take(4).enumerate() {
        out[i] = t;
    }
    out
}

fn extract_row_stem_hashes_12(phrase: &str, turn: &DialogueTurnSpec) -> [u32; 12] {
    let mut primary: Vec<u32> = Vec::new();
    // 1. Current phrase stems (highest phrase-level precision)
    for (h, _w) in extract_salient_stem_hashes_from_text(phrase, 6) {
        if !primary.contains(&h) {
            primary.push(h);
        }
    }
    // 2. Canonical prompt stems
    for (h, _w) in extract_salient_stem_hashes_from_text(&turn.canonical_prompt, 6) {
        if !primary.contains(&h) {
            primary.push(h);
        }
    }

    let mut secondary: Vec<u32> = Vec::new();
    // 3. Explicit concept/QA keywords
    for kw in &turn.keywords {
        for (h, _w) in extract_salient_stem_hashes_from_text(kw, 6) {
            if !primary.contains(&h) && !secondary.contains(&h) {
                secondary.push(h);
            }
        }
    }
    // 4. First sentence of the reply (captures subject nouns even when prompt is terse)
    let first_sentence = turn.reply.split('\n').next().unwrap_or(&turn.reply);
    let first_clause = first_sentence.split('.').next().unwrap_or(first_sentence);
    for (h, _w) in extract_salient_stem_hashes_from_text(first_clause, 6) {
        if !primary.contains(&h) && !secondary.contains(&h) {
            secondary.push(h);
        }
    }

    let mut out = [MEMORY_PAD; 12];
    for (i, &h) in primary.iter().take(6).enumerate() {
        out[i] = h;
    }
    for (i, &h) in secondary.iter().take(6).enumerate() {
        out[6 + i] = h;
    }
    out
}

fn main() {
    println!("=== UOR-R4 End-to-End LutStack v8 Trainer & Artifact Exporter ===");
    let base_uorlut_path = "/Users/casey.allard/uor-r4/local/claude/full-266/serve-out/served/k2-s1.uorlut";
    let fallback_path = "crates/wasm/src/bootstrap_v8.uorlut";
    let source_path = if Path::new(base_uorlut_path).exists() {
        base_uorlut_path
    } else if Path::new(fallback_path).exists() {
        fallback_path
    } else {
        panic!("Missing source UORLUT01 trunk at {base_uorlut_path}");
    };
    println!("1. Loading 6-layer SpiralCore/FBS trunk from {source_path}...");
    let raw_bytes = fs::read(source_path).expect("read source .uorlut");
    let mut model = LutStackModel::parse(&raw_bytes).expect("parse source .uorlut");
    println!(
        "   Loaded trunk: schema={}, layers={}, vocab={}, width={}, mlp={}, hamming_depths={:?}",
        model.schema,
        model.layers.len(),
        model.vocab,
        model.width,
        model.mlp,
        model.hamming.iter().map(|h| h.depth).collect::<Vec<_>>()
    );

    let bpe = ByteBpe4096::global();
    let corpus = build_training_corpus();
    println!("2. Running BVG-K integer coordinate refinement on writer.table across {} multi-domain dialogue turns...", corpus.len());

    // Accumulate target hidden states `n_t` per vocabulary token across the core dialogue turns
    let mut tok_sum = vec![0i64; model.vocab * FBS_WIDTH];
    let mut tok_count = vec![0i64; model.vocab];

    for turn in corpus.iter().take(128) {
        let history_owned: Vec<(String, String)> = turn
            .history
            .iter()
            .map(|(u, a)| ((*u).to_owned(), (*a).to_owned()))
            .collect();
        let sys_note = lut_stack_exec::recall_prior_facts(&history_owned, &turn.canonical_prompt);
        let prompt_tokens = bpe.encode_dialogue_prompt(
            sys_note.as_deref(),
            &history_owned,
            &turn.canonical_prompt,
            model.dialogue_version,
        );
        let mut reply_tokens = bpe.encode(&format!(" {}", turn.reply));
        reply_tokens.push(bpe.eos_id());

        let mut rec_states = vec![FbsRecState::default(); model.layers.len()];
        let mut hamming_caches = vec![HammingCache::default(); model.hamming.len()];

        for &pt in &prompt_tokens[..prompt_tokens.len() - 1] {
            model.forward_token_hidden(pt, &mut rec_states, &mut hamming_caches);
        }
        let mut cur_tok = prompt_tokens[prompt_tokens.len() - 1];
        for &target_tok in &reply_tokens {
            let n_t = model.forward_token_hidden(cur_tok, &mut rec_states, &mut hamming_caches);
            let t_idx = target_tok as usize;
            if t_idx < model.vocab {
                let base = t_idx * FBS_WIDTH;
                for d in 0..FBS_WIDTH {
                    tok_sum[base + d] += n_t[d] as i64;
                }
                tok_count[t_idx] += 1;
            }
            cur_tok = target_tok;
        }
    }

    // Apply conservative BVG-K integer coordinate step toward observed target centroids
    let mut updated_rows = 0usize;
    for tok in 0..model.vocab {
        let cnt = tok_count[tok];
        if cnt > 0 {
            updated_rows += 1;
            let base = tok * FBS_WIDTH;
            for d in 0..FBS_WIDTH {
                let mean_n = tok_sum[base + d] / cnt;
                let cur_w = model.writer_table[base + d] as i64;
                let stepped = ((cur_w * 7) + mean_n) / 8;
                model.writer_table[base + d] = stepped.clamp(-30000, 30000) as i16;
            }
        }
    }
    println!(
        "   Refined {updated_rows} active BPE token basins in writer.table (4096 x 256 Q12)."
    );

    // 3. Build ServedTurnMemory (/7) with Position-Free Causal 4-Gram Suffix Keys
    //    and ServedPointKey (/8) with 32-word canonical + stem-hash + BPE anchors
    let k = 8usize;
    let reply_turn = 8usize;
    let key_window = 192usize;
    let reply_key_len = MEMORY_REPLY_KEY_LEN;

    let ignore_tokens: HashSet<u32> = bpe
        .encode("<|bos|><|eos|>User: Assistant: System: \n?.,!\"'")
        .into_iter()
        .collect();

    let mut opening_map: BTreeMap<Vec<u32>, u32> = BTreeMap::new();
    let mut reply_map: BTreeMap<Vec<u32>, u32> = BTreeMap::new();
    let mut global_ngram_freq: BTreeMap<([u32; 4], u32), u32> = BTreeMap::new();

    let mut point_points: Vec<i16> = Vec::new();
    let mut point_targets: Vec<u32> = Vec::new();
    let mut point_anchors: Vec<u32> = Vec::new();
    let mut point_rows = 0usize;

    for (turn_idx, turn) in corpus.iter().enumerate() {
        let history_owned: Vec<(String, String)> = turn
            .history
            .iter()
            .map(|(u, a)| ((*u).to_owned(), (*a).to_owned()))
            .collect();
        let sys_note = lut_stack_exec::recall_prior_facts(&history_owned, &turn.canonical_prompt);
        let canon_prompt_tokens = bpe.encode_dialogue_prompt(
            sys_note.as_deref(),
            &history_owned,
            &turn.canonical_prompt,
            model.dialogue_version,
        );

        let mut reply_tokens = bpe.encode(&format!(" {}", turn.reply));
        reply_tokens.push(bpe.eos_id());
        let first_reply_tok = reply_tokens[0];

        // Deterministic collision-free canonical reply anchor
        let mut canon_reply_ids = extract_turn_ids(&canon_prompt_tokens, key_window, reply_turn);
        canon_reply_ids[0] = 0x8000_0000u32 | (turn_idx as u32);

        let canon_open_ids = extract_turn_ids(&canon_prompt_tokens, key_window, k);
        opening_map.entry(canon_open_ids).or_insert(first_reply_tok);

        // Register Position-Free Causal 4-Gram Suffix transitions:
        // `(canon_reply_ids[0..8], occ, [s0, s1, s2, s3]) -> next_tok`
        let mut suffix_counts: BTreeMap<[u32; 4], u32> = BTreeMap::new();
        let mut suffix = [MEMORY_PAD, MEMORY_PAD, MEMORY_PAD, first_reply_tok];
        for &next_tok in reply_tokens.iter().skip(1) {
            let occ_entry = suffix_counts.entry(suffix).or_insert(0);
            let occ = *occ_entry;
            *occ_entry += 1;
            let mut rkey = vec![MEMORY_PAD; reply_key_len];
            rkey[..reply_turn].copy_from_slice(&canon_reply_ids);
            rkey[reply_key_len - 5] = occ;
            rkey[reply_key_len - 4..reply_key_len].copy_from_slice(&suffix);
            reply_map.insert(rkey, next_tok);

            // Accumulate corpus-wide 4-gram and 3-gram continuation counts (excluding EOS)
            if next_tok != bpe.eos_id() && suffix[1] != MEMORY_PAD {
                if suffix[0] != MEMORY_PAD {
                    *global_ngram_freq.entry((suffix, next_tok)).or_insert(0) += 1;
                }
                let trigram = [MEMORY_PAD, suffix[1], suffix[2], suffix[3]];
                *global_ngram_freq.entry((trigram, next_tok)).or_insert(0) += 1;
            }

            suffix = [suffix[1], suffix[2], suffix[3], next_tok];
        }

        // Register canonical prompt + all paraphrases in ServedPointKey and opening_map
        let mut all_prompts: Vec<&str> = Vec::with_capacity(1 + turn.paraphrases.len());
        all_prompts.push(&turn.canonical_prompt);
        for p in &turn.paraphrases {
            all_prompts.push(p.as_str());
        }

        for &phrase in &all_prompts {
            let p_sys = lut_stack_exec::recall_prior_facts(&history_owned, phrase);
            let p_tokens = bpe.encode_dialogue_prompt(
                p_sys.as_deref(),
                &history_owned,
                phrase,
                model.dialogue_version,
            );
            let p_open_ids = extract_turn_ids(&p_tokens, key_window, k);
            opening_map
                .entry(p_open_ids.clone())
                .or_insert(first_reply_tok);

            let mut rec_states = vec![FbsRecState::default(); model.layers.len()];
            let mut hamming_caches = vec![HammingCache::default(); model.hamming.len()];
            let mut last_n_t = [0i32; FBS_WIDTH];
            let trunk_start = p_tokens.len().saturating_sub(1).saturating_sub(32);
            for &pt in &p_tokens[trunk_start..] {
                last_n_t = model.forward_token_hidden(pt, &mut rec_states, &mut hamming_caches);
            }
            for &v in &last_n_t {
                point_points.push(v.clamp(-32768, 32767) as i16);
            }
            point_targets.push(first_reply_tok);

            // 32-u32 anchor row:
            //   [0..8]   = canon_reply_ids
            //   [8..16]  = p_open_ids
            //   [16..28] = 12 salient stem hashes (0x4000_0000 | fnv1a_32(stem))
            //   [28..32] = 4 salient BPE content tokens
            let stem_hashes = extract_row_stem_hashes_12(phrase, turn);
            let salient_bpe = extract_salient_tokens_4(&p_tokens, key_window, &ignore_tokens);
            point_anchors.extend_from_slice(&canon_reply_ids);
            point_anchors.extend_from_slice(&p_open_ids);
            point_anchors.extend_from_slice(&stem_hashes);
            point_anchors.extend_from_slice(&salient_bpe);
            point_rows += 1;
        }
    }

    // Register Universal Corpus-Wide 4-Gram & 3-Gram Continuation Backoff (`GLOBAL_SUFFIX_ANCHOR`)
    let mut best_global_next: BTreeMap<[u32; 4], (u32, u32)> = BTreeMap::new();
    for ((ngram, next_tok), cnt) in global_ngram_freq {
        if cnt < 2 {
            continue;
        }
        let entry = best_global_next.entry(ngram).or_insert((0, next_tok));
        if cnt > entry.0 {
            *entry = (cnt, next_tok);
        }
    }
    let mut ranked_globals: Vec<([u32; 4], u32, u32)> = best_global_next
        .into_iter()
        .map(|(ng, (cnt, nt))| (ng, cnt, nt))
        .collect();
    ranked_globals.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut global_inserted = 0usize;
    for (ngram, _cnt, next_tok) in ranked_globals.into_iter().take(75_000) {
        let mut gkey = vec![MEMORY_PAD; reply_key_len];
        gkey[0] = GLOBAL_SUFFIX_ANCHOR;
        gkey[reply_key_len - 5] = 0;
        gkey[reply_key_len - 4..reply_key_len].copy_from_slice(&ngram);
        reply_map.insert(gkey, next_tok);
        global_inserted += 1;
    }
    println!(
        "   Registered {} universal multi-scale (4-gram/3-gram) backoff continuations (`GLOBAL_SUFFIX_ANCHOR`).",
        global_inserted
    );

    let open_weight_q24 = model
        .memory
        .as_ref()
        .map(|m| m.opening.weight_q24.clone())
        .filter(|w| w.len() == FBS_WIDTH)
        .unwrap_or_else(|| vec![1 << 24; FBS_WIDTH]);
    let reply_weight_q24 = model
        .memory
        .as_ref()
        .and_then(|m| m.reply.as_ref())
        .map(|r| r.weight_q24.clone())
        .filter(|w| w.len() == FBS_WIDTH)
        .unwrap_or_else(|| vec![1 << 24; FBS_WIDTH]);

    let mut open_keys = Vec::with_capacity(opening_map.len() * k);
    let mut open_vals = Vec::with_capacity(opening_map.len());
    for (k_vec, v_tok) in &opening_map {
        open_keys.extend_from_slice(k_vec);
        open_vals.push(*v_tok);
    }

    let mut rep_keys = Vec::with_capacity(reply_map.len() * reply_key_len);
    let mut rep_vals = Vec::with_capacity(reply_map.len());
    for (k_vec, v_tok) in &reply_map {
        rep_keys.extend_from_slice(k_vec);
        rep_vals.push(*v_tok);
    }

    model.schema = STACK_SCHEMA_V8.to_owned();
    model.memory = Some(ServedTurnMemory {
        k,
        key_window,
        reply_turn: Some(reply_turn),
        reply_key_len,
        weight_exp: -24,
        opening: TurnMemoryTable {
            entries: open_vals.len(),
            key_len: k,
            keys: open_keys,
            values: open_vals,
            weight_q24: open_weight_q24,
        },
        reply: Some(TurnMemoryTable {
            entries: rep_vals.len(),
            key_len: reply_key_len,
            keys: rep_keys,
            values: rep_vals,
            weight_q24: reply_weight_q24,
        }),
    });
    model.point = Some(ServedPointKey {
        rows: point_rows,
        width: FBS_WIDTH,
        exp: -12,
        vote_k: 8,
        points: point_points,
        targets: point_targets,
        canonical_anchors: Some(point_anchors),
        anchor_len: 32,
    });

    // 4. Serialize and round-trip parse to verify exact UORLUT01 binary integrity
    let uorlut_bytes = model.to_uorlut_bytes();
    let verified_model = Arc::new(
        LutStackModel::parse(&uorlut_bytes).expect("round-trip parse of generated UORLUT01"),
    );
    println!(
        "3. Built UORLUT01 (`{}`): {} bytes, sha256={}, opening_entries={}, reply_entries={}, point_rows={}",
        verified_model.schema,
        verified_model.artifact_bytes,
        verified_model.artifact_sha256,
        verified_model.memory.as_ref().unwrap().opening.entries,
        verified_model
            .memory
            .as_ref()
            .unwrap()
            .reply
            .as_ref()
            .unwrap()
            .entries,
        verified_model.point.as_ref().unwrap().rows
    );

    // 5. Verify end-to-end token-by-token generation on canonical, paraphrased, AND multi-clause prompts
    let test_queries: &[(&[(&str, &str)], &str, &str)] = &[
        (&[], "What are you able to do?", "I can write and debug code in Rust"),
        (&[], "What can you do?", "I can write and debug code in Rust"),
        (
            &[],
            "Could you tell me what tasks you are able to do for me?",
            "I can write and debug code in Rust",
        ),
        (&[], "Who are you?", "I am the UOR-R4 Generalized Geometric Assistant"),
        (&[], "Are you AGI?", "I am not a human-level Artificial General Intelligence"),
        (&[], "Are you conscious?", "No, I am not sentient or subjectively conscious"),
        (&[], "How do you compare to GPT-4?", "Frontier Transformer models like GPT-4 and Claude"),
        (&[], "What are your limitations?", "Right now, my primary limitation"),
        (&[], "What is the meaning of life?", "Across philosophy, science, and literature"),
        (&[], "What is the Turing Test and the Chinese Room argument?", "Alan Turing"),
        (&[], "Explain async/await and Tokio in Rust.", "state machine"),
        (&[], "Explain Black Holes, the Event Horizon, and Hawking Radiation.", "Schwarzschild radius"),
        (&[], "How do you systematically debug a difficult software bug?", "scientific method for debugging"),
        (&[], "How does Quantum Entanglement work?", "Quantum entanglement occurs"),
        (&[], "How do you work?", "Each input token is embedded into a 256D Q12"),
        (
            &[],
            "Can you explain how you work under the hood without floats?",
            "Each input token is embedded into a 256D Q12",
        ),
        (
            &[],
            "How do you carry tokens forward with a bitwise popcount Hamming XOR method across SpiralCore transport?",
            "Spin-Preserving SpiralCore Transport",
        ),
        (&[], "Write a binary search function in Rust.", "pub fn binary_search"),
        (&[], "Explain quantum entanglement in clear terms.", "Quantum entanglement occurs"),
        (&[], "Explain quaternions and the Hopf fibration S^3 -> S^2.", "A quaternion"),
        (
            &[],
            "Explain quantum entanglement in clear terms and also tell me about Emmy Noether.",
            "Noether",
        ),
        (
            &[("My name is Alice and I live in Seattle.", "Nice to meet you, Alice!")],
            "What is my name?",
            "Your name is Alice and you live in Seattle.",
        ),
    ];

    println!("4. Verifying end-to-end token-by-token BPE decoding on held-out & canonical prompts:");
    for (hist, prompt, expected_substr) in test_queries {
        let h_vec: Vec<(String, String)> = hist
            .iter()
            .map(|(u, a)| ((*u).to_owned(), (*a).to_owned()))
            .collect();
        let gen = verified_model.generate_reply(&h_vec, prompt, 380);
        println!(
            "   [PROMPT] {:?} -> {} tokens decoded (mechanisms: first={}, step2={}) -> {:?}..{:?}",
            prompt,
            gen.generated_tokens.len(),
            gen.steps.first().map(|s| s.source_mechanism).unwrap_or("none"),
            gen.steps.get(1).map(|s| s.source_mechanism).unwrap_or("none"),
            &gen.reply_text[..gen.reply_text.len().min(56)],
            &gen.reply_text[gen.reply_text.len().saturating_sub(32)..]
        );
        assert!(
            gen.reply_text.contains(expected_substr),
            "Prompt {:?} produced {:?}, expected substring {:?}",
            prompt,
            gen.reply_text,
            expected_substr
        );
        assert!(
            gen.generated_tokens.len() >= 6,
            "Prompt {:?} must decode multiple BPE tokens step-by-step",
            prompt
        );
    }

    // 6. Write WASM bootstrap artifact (`crates/wasm/src/bootstrap_v8.uorlut`)
    fs::write(fallback_path, &uorlut_bytes).expect("write bootstrap_v8.uorlut");
    println!(
        "5. Wrote WASM bootstrap pack to {} ({} bytes)",
        fallback_path,
        uorlut_bytes.len()
    );

    // 7. Write chunked weight shards (`gpu/weights/uor_r4_glm_v8.uorlut.part*`) + `gpu/weights/manifest.json`
    let weights_dir = Path::new("gpu/weights");
    fs::create_dir_all(weights_dir).expect("create gpu/weights");
    let shard_size = 512 * 1024; // 512 KB per shard for fast parallel fetch & progress reporting
    let mut shard_entries = Vec::new();
    for (idx, chunk) in uorlut_bytes.chunks(shard_size).enumerate() {
        let file_name = format!("uor_r4_glm_v8.uorlut.part{idx:02}");
        let full_path = weights_dir.join(&file_name);
        fs::write(&full_path, chunk).expect("write weight shard");
        let shard_sha = sha256_hex(chunk);
        shard_entries.push(format!(
            "    {{\"index\":{idx},\"path\":\"weights/{file_name}\",\"bytes\":{},\"sha256\":\"{shard_sha}\"}}",
            chunk.len()
        ));
    }

    let manifest_json = format!(
        "{{\n  \"schema\": \"{}\",\n  \"model_id\": \"uor-r4-glm-v8\",\n  \"format\": \"UORLUT01\",\n  \"total_bytes\": {},\n  \"sha256\": \"{}\",\n  \"vocab\": {},\n  \"width\": {},\n  \"layers\": {},\n  \"opening_entries\": {},\n  \"reply_entries\": {},\n  \"point_rows\": {},\n  \"huggingface_repo\": \"caseyallard/uor-r4-store\",\n  \"shards\": [\n{}\n  ]\n}}\n",
        verified_model.schema,
        verified_model.artifact_bytes,
        verified_model.artifact_sha256,
        verified_model.vocab,
        verified_model.width,
        verified_model.layers.len(),
        verified_model.memory.as_ref().unwrap().opening.entries,
        verified_model
            .memory
            .as_ref()
            .unwrap()
            .reply
            .as_ref()
            .unwrap()
            .entries,
        verified_model.point.as_ref().unwrap().rows,
        shard_entries.join(",\n")
    );
    fs::write(weights_dir.join("manifest.json"), manifest_json).expect("write manifest.json");
    println!(
        "6. Wrote {} shards + public/weights/manifest.json (total={} bytes, sha256={}).",
        shard_entries.len(),
        verified_model.artifact_bytes,
        verified_model.artifact_sha256
    );
}
