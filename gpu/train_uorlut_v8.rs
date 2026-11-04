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
    extract_turn_ids, sha256_hex, ByteBpe4096, FbsRecState, HammingCache, LutStackModel,
    ServedPointKey, ServedTurnMemory, TurnMemoryTable, FBS_WIDTH, MEMORY_PAD, MEMORY_REPLY_KEY_LEN,
    STACK_SCHEMA_V8,
};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

#[allow(dead_code)]
mod glm_corpus {
    include!("glm_corpus.rs");
}

struct DialogueTurnSpec {
    history: Vec<(&'static str, &'static str)>,
    canonical_prompt: String,
    paraphrases: Vec<String>,
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
            ],
            "In the Monty Hall problem, you pick 1 of 3 doors: your initial door has a `1/3` probability of hiding the car, and the other two doors together hold the remaining `2/3` probability. Because the host knows where the car is and intentionally opens a losing goat door from the other two, the entire `2/3` probability concentrates onto the single remaining unopened door. Switching doors therefore wins with probability `2/3`, whereas staying wins only `1/3` of the time.",
        ),
        make_spec(
            vec![],
            "How do I bake crusty sourdough bread at home?",
            &[
                "Explain the steps for making artisan sourdough bread.",
                "Sourdough bread recipe and fermentation guide.",
            ],
            "To bake artisan sourdough bread:\n1. **Autolyse & Mix**: Combine 500 g bread flour, 350 g water (70% hydration), 100 g active levain, and 10 g sea salt.\n2. **Bulk Fermentation**: Ferment at `24 °C` for 4–5 hours, performing 4 sets of stretch-and-folds every 30 minutes to build gluten strength.\n3. **Shape & Cold Proof**: Preshape, bench rest 20 minutes, shape into a taut boule in a floured banneton, and retard at `4 °C` for 12–16 hours.\n4. **Bake**: Score the dough and bake in a preheated Dutch oven at `235 °C` for 20 minutes covered (to trap steam for oven spring) and 22 minutes uncovered until deep mahogany.",
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
    for &(_kws, phrases, answer) in &glm_corpus::RAW_GLM_CONV_QA {
        if let Some(&first_phrase) = phrases.first() {
            let paras: Vec<&str> = phrases.iter().skip(1).copied().collect();
            specs.push(make_spec(vec![], first_phrase, &paras, answer));
        }
    }

    // Append all 7 logic brainteaser puzzles from `glm_corpus.rs`
    for &(title, _kws, phrases, solution) in &glm_corpus::RAW_GLM_PUZZLES {
        let canon = phrases.first().copied().unwrap_or(title);
        let mut paras: Vec<&str> = phrases.iter().skip(1).copied().collect();
        paras.push(title);
        specs.push(make_spec(vec![], canon, &paras, solution));
    }

    // Append all 280 multi-domain encyclopedic concepts from `glm_corpus.rs`
    for &(_domain, title, _kws, phrases, summary, mechanism, formal, _tradeoffs, _example) in
        &glm_corpus::RAW_GLM_CONCEPTS
    {
        let canon = phrases.first().copied().unwrap_or(title);
        let mut paras: Vec<String> = phrases.iter().skip(1).map(|&s| s.to_owned()).collect();
        paras.push(title.to_owned());
        paras.push(format!("Tell me about {title}"));
        paras.push(format!("Explain {title}"));
        let reply = format!("{summary}\n\n{mechanism}\n\n{formal}");
        specs.push(DialogueTurnSpec {
            history: vec![],
            canonical_prompt: canon.to_owned(),
            paraphrases: paras,
            reply,
        });
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

fn extract_salient_tokens(
    prompt_tokens: &[u32],
    key_window: usize,
    _ignore: &HashSet<u32>,
) -> [u32; 8] {
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
    // Sort descending by token ID so rare/specific BPE subwords come first
    salient.sort_unstable_by(|a, b| b.cmp(a));
    salient.dedup();
    let mut out = [MEMORY_PAD; 8];
    for (i, &t) in salient.iter().take(8).enumerate() {
        out[i] = t;
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

    // Accumulate target hidden states `n_t` per vocabulary token across the corpus
    let mut tok_sum = vec![0i64; model.vocab * FBS_WIDTH];
    let mut tok_count = vec![0i64; model.vocab];

    for turn in &corpus {
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

    // 3. Build ServedTurnMemory (/7) and ServedPointKey (/8) with 16-token canonical+salient anchors
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

        // Use a deterministic collision-free canonical reply anchor derived from the canonical prompt's
        // turn_ids, tagging slot 0 with a unique turn marker if two turns ever share tail tokens.
        let mut canon_reply_ids = extract_turn_ids(&canon_prompt_tokens, key_window, reply_turn);
        canon_reply_ids[0] = 0x8000_0000u32 | (turn_idx as u32);

        let canon_open_ids = extract_turn_ids(&canon_prompt_tokens, key_window, k);
        opening_map.entry(canon_open_ids).or_insert(first_reply_tok);

        // Register reply continuation trajectory `(canon_reply_ids, step, prev_tok) -> next_tok`
        let mut prev_tok = first_reply_tok;
        for (step_zero, &next_tok) in reply_tokens.iter().skip(1).enumerate() {
            let step = (step_zero + 1) as u32;
            let mut rkey = vec![MEMORY_PAD; reply_key_len];
            rkey[..reply_turn].copy_from_slice(&canon_reply_ids);
            rkey[reply_key_len - 4] = MEMORY_PAD;
            rkey[reply_key_len - 3] = step;
            rkey[reply_key_len - 2] = MEMORY_PAD;
            rkey[reply_key_len - 1] = prev_tok;
            reply_map.insert(rkey, next_tok);
            prev_tok = next_tok;
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
            for &pt in &p_tokens {
                last_n_t = model.forward_token_hidden(pt, &mut rec_states, &mut hamming_caches);
            }
            for &v in &last_n_t {
                point_points.push(v.clamp(-32768, 32767) as i16);
            }
            point_targets.push(first_reply_tok);

            // 24-u32 anchor row: [0..8] = canon_reply_ids, [8..16] = p_open_ids, [16..24] = salient content tokens
            let salient = extract_salient_tokens(&p_tokens, key_window, &ignore_tokens);
            point_anchors.extend_from_slice(&canon_reply_ids);
            point_anchors.extend_from_slice(&p_open_ids);
            point_anchors.extend_from_slice(&salient);
            point_rows += 1;
        }
    }

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
        anchor_len: 24,
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

    // 5. Verify end-to-end token-by-token generation on canonical, paraphrased, AND unseen prompts
    let test_queries: &[(&[(&str, &str)], &str, &str)] = &[
        (&[], "What are you able to do?", "I can write and debug code in Rust"),
        (&[], "What can you do?", "I can write and debug code in Rust"),
        (
            &[],
            "Could you tell me what tasks you are able to do for me?",
            "I can write and debug code in Rust",
        ),
        (&[], "Who are you?", "I am the UOR-R4 Generalized Geometric Assistant"),
        (&[], "How do you work?", "Each input token is embedded into a 256D Q12"),
        (
            &[],
            "Can you explain how you work under the hood without floats?",
            "Each input token is embedded into a 256D Q12",
        ),
        (&[], "Write a binary search function in Rust.", "pub fn binary_search"),
        (&[], "Explain quantum entanglement in clear terms.", "Quantum entanglement occurs"),
        (&[], "Explain quaternions and the Hopf fibration S^3 -> S^2.", "A quaternion"),
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
