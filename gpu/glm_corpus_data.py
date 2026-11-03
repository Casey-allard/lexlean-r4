"""Multi-domain concept & reasoning corpus for the UOR-R4 Generalized Geometric Language Model (GLM)."""

# Each entry is:
# (domain, title, keywords, phrases, summary, mechanism, formal_principle, tradeoffs, example)

GLM_CONCEPTS_PART1 = [
    # ==================== COMPUTER SCIENCE & ALGORITHMS ====================
    (
        "Computer Science · Architecture",
        "UOR-R4 Geometric Language Model vs. Transformers",
        ["uorr4", "transformer", "transformers", "attention", "matmul", "geometric", "llm", "glms"],
        ["uor-r4", "geometric language model", "self attention", "matrix multiplication"],
        "UOR-R4 is a float-free, zero-matmul Geometric Language Model (GLM) that replaces dense floating-point self-attention (`QK^T`) with exact discrete group transport over the 16-root `E8` orbit, the 64-action `Cl(0,6)` Clifford carrier, the 17-root Icosian `Z[phi]` lattice, and `/6..8` associative lookup tables.",
        "Instead of computing `O(N^2 * d)` floating-point dot products per token, UOR-R4 maps every token to an 8-lane `Q12` integer coordinate vector and a 64-bit unit-quaternion rotated sign code. Context routing uses 8-head cyclic flock consensus (`R4.Flock`) and `RETAIN_LAMBDA_Q31` geometric retention (`1 - 2^-(k+1)`), while recall combines `/6` SWAR Hamming induction, `/7` exact n-gram continuation tables, and `/8` continuous `Q12` L1 nearest-neighbor voting.",
        "Complexity: `O(1)` table lookup + `O(K)` SWAR popcount scan with zero floating-point rounding drift (`a + (b + c) = (a + b) + c`), verified across 27 zero-axiom Lean 4.32.1 conformance suites (`R4-01..R4-27`).",
        "Transformers excel at brute-force continuous interpolation when trained on thousands of GPUs, but suffer from quadratic context cost, floating-point non-determinism, and arithmetic hallucinations. UOR-R4 provides deterministic bit-exact execution, cryptographic SHA-256/BLAKE3 receipts, and instant on-device discrete learning (`evaluateCoupledGateVector`).",
        "When evaluating `48291 + 73948` or routing multi-turn context in WebAssembly, UOR-R4 dispatches through the verified `R4.ExactArithmetic` (M6) and `R4.Flock` kernels in sub-millisecond integer arithmetic.",
    ),
    (
        "Computer Science · Data Structures",
        "Hash Tables & Consistent Hashing",
        ["hash", "hashtable", "hashmap", "dict", "dictionary", "collision", "chaining", "probing", "consistent"],
        ["hash table", "hash map", "consistent hashing", "open addressing", "robin hood hashing"],
        "A hash table is an associative data structure that maps keys to values in amortized `O(1)` time by applying a deterministic hash function `h(k)` to map each key into an array bucket index `h(k) mod M`.",
        "When two distinct keys map to the same bucket (a collision), implementations resolve it via either Separate Chaining (linked lists or small vectors per bucket) or Open Addressing (linear/quadratic probing or Robin Hood hashing, which minimizes probe-length variance by swapping keys based on distance from their initial bucket). In distributed caches, Consistent Hashing maps both servers and keys onto a `2^64` ring with virtual nodes so adding/removing a node only remaps `O(K / N)` keys.",
        "Average time complexity: `O(1)` lookup, insertion, and deletion when load factor `alpha = N / M < 0.75`; worst-case `O(N)` under adversarial hash collisions unless keyed hashes (`SipHash`) are used.",
        "Hash tables offer unbeatable `O(1)` point lookups but do not preserve key ordering (unlike B-Trees), making range scans (`WHERE k BETWEEN a AND b`) require `O(N)` full-table iteration.",
        "Rust's `std::collections::HashMap` uses Google's SwissTable (`hashbrown`) with SIMD 1-byte control tags (`H2` fingerprint) to probe 16 slots in a single CPU instruction.",
    ),
    (
        "Computer Science · Storage Engines",
        "B-Trees vs. LSM-Trees (Log-Structured Merge-Trees)",
        ["btree", "btrees", "b+tree", "lsm", "sstable", "memtable", "compaction", "indexing"],
        ["b-tree", "b+ tree", "lsm tree", "lsm-tree", "log structured merge"],
        "B-Trees (and `B+Trees`) and Log-Structured Merge-Trees (`LSM-Trees`) are the two foundational on-disk storage engines powering modern relational and NoSQL databases.",
        "A `B+Tree` stores keys in sorted, fixed-size disk pages (`4 KB - 16 KB`) with high fanout (`B = 100..500`), keeping internal routing keys in upper nodes and all row payloads in doubly-linked leaf pages for fast `O(log_B N)` point and range reads. An `LSM-Tree` buffers all incoming writes sequentially into an in-memory Write-Ahead Log (`WAL`) and a sorted `MemTable` (SkipList/B-Tree), flushing full MemTables into immutable on-disk `SSTables` (Sorted String Tables) with Bloom filters and background level compaction.",
        "B+Tree: `O(log_B N)` read and write I/O (in-place random page writes). LSM-Tree: `O(1)` sequential write I/O (`MemTable + WAL`) and `O(L)` read lookup across levels, accelerated to `~O(1)` disk hits by per-SSTable Bloom filters.",
        "B+Trees dominate read-heavy transactional workloads (PostgreSQL, MySQL InnoDB, SQLite) with lower read amplification, whereas LSM-Trees dominate write-intensive workloads (RocksDB, LevelDB, Cassandra, ScyllaDB) with much lower write amplification and higher SSD sequential throughput.",
        "In RocksDB, a write appends to the WAL and inserts into a SkipList `MemTable` in microseconds; reads check the `MemTable` first, then consult Bloom filters across `L0..L6` SSTables.",
    ),
    (
        "Computer Science · Graph Algorithms",
        "Graph Traversal & Shortest Paths (BFS, DFS, Dijkstra & A*)",
        ["graph", "bfs", "dfs", "dijkstra", "astar", "shortest", "topological", "spanning", "kruskal", "prim"],
        ["shortest path", "breadth first", "depth first", "dijkstra algorithm", "a* search", "topological sort"],
        "Graph traversal and shortest-path algorithms systematically explore vertices `V` and edges `E` to discover connectivity, dependency orders, and minimum-cost trajectories.",
        "Breadth-First Search (`BFS`) explores level-by-level using a FIFO queue, finding shortest paths in unweighted graphs in `O(V + E)`. Depth-First Search (`DFS`) explores deep branches via a LIFO stack/recursion, enabling cycle detection, strongly connected components (Tarjan/Kosaraju), and topological sorting of DAGs. For non-negative edge weights, Dijkstra's algorithm greedily expands the frontier using a min-heap priority queue on tentative distance `d[u] + w(u, v)`. `A*` search augments Dijkstra with an admissible heuristic `h(v) <= dist(v, goal)`, prioritizing `f(v) = g(v) + h(v)`.",
        "BFS/DFS: `O(V + E)` time, `O(V)` space. Dijkstra with binary heap: `O((V + E) log V)` time. Bellman-Ford (handles negative weights & detects negative cycles): `O(V * E)` time.",
        "BFS guarantees shortest unweighted paths but uses `O(b^d)` frontier memory; DFS uses only `O(d)` stack memory but is not shortest-path optimal; Dijkstra fails on negative edge weights (requiring Bellman-Ford); A* is fastest when a tight, consistent heuristic (like Euclidean/Manhattan distance) is available.",
        "Routing packets across OSPF network links uses Dijkstra's shortest-path tree, while build systems like Cargo and Bazel use DFS/Kahn's algorithm for topological dependency ordering.",
    ),
    (
        "Computer Science · Algorithms",
        "Dynamic Programming, Memoization & Greedy Algorithms",
        ["dynamic", "memoization", "tabulation", "subproblem", "subproblems", "knapsack", "greedy", "bellman", "editdistance", "lcs"],
        ["dynamic programming", "optimal substructure", "overlapping subproblems", "greedy algorithm", "longest common subsequence"],
        "Dynamic Programming (`DP`) solves complex optimization and counting problems by decomposing them into overlapping subproblems that satisfy the Bellman principle of optimal substructure.",
        "Top-Down Memoization writes the natural recursive relation and caches computed states in a hash table or array so each distinct state is evaluated exactly once. Bottom-Up Tabulation orders states topologically (from base cases to the target) and fills an iterative DP table, often reducing space complexity from `O(N * M)` to `O(M)` via rolling rows. By contrast, a Greedy Algorithm makes the locally optimal choice at each step without backtracking, which is globally optimal only when the problem satisfies the matroid or greedy-choice property (e.g. Huffman coding, Kruskal/Prim MST, interval scheduling).",
        "Bellman Equation: `V(s) = min_{a in A(s)} [ cost(s, a) + V(next(s, a)) ]`. Example: 0/1 Knapsack runs in `O(N * W)` pseudo-polynomial time; Levenshtein Edit Distance runs in `O(N * M)` time and `O(min(N, M))` space.",
        "Greedy algorithms are faster (`O(N log N)`) and simpler when local optimality implies global optimality, but fail on problems like 0/1 Knapsack or arbitrary coin systems (`{1, 3, 4}` for target `6`, where greedy picks `4+1+1` instead of `3+3`), where DP guarantees the exact global optimum.",
        "Computing Longest Common Subsequence (`LCS`) for `git diff` uses recurrence `dp[i][j] = dp[i-1][j-1] + 1` if `a[i] == b[j]` else `max(dp[i-1][j], dp[i][j-1])`.",
    ),
    (
        "Computer Science · Algorithms",
        "Sorting Algorithms (QuickSort, MergeSort, HeapSort & Radix Sort)",
        ["sort", "sorting", "quicksort", "mergesort", "heapsort", "radixsort", "timsort", "partition"],
        ["sorting algorithm", "merge sort", "quick sort", "heap sort", "radix sort", "time complexity"],
        "Sorting algorithms arrange `N` elements into total order using either comparison-based partitioning/merging (`Omega(N log N)` lower bound) or non-comparison radix digit bucketing (`O(N * k)`).",
        "QuickSort partitions an array in-place around a pivot (`3-way Dutch National Flag` split into `< pivot`, `== pivot`, `> pivot`) and recurses on the sub-arrays. MergeSort recursively splits the array in half and merges sorted runs stably in `O(N log N)` worst-case time. HeapSort builds an in-place binary max-heap in `O(N)` and repeatedly extracts the maximum in `O(N log N)` with `O(1)` auxiliary space. Radix Sort distributes integers digit-by-digit (`LSD` or `MSD`) into radix buckets without element comparisons.",
        "Comparison sort information-theoretic lower bound: `log2(N!) = N log2(N) - O(N)` comparisons. QuickSort/MergeSort/HeapSort/TimSort: `O(N log N)` average; QuickSort is `O(N)` space-free (`O(log N)` stack); Radix Sort: `O(N * (w / r))`.",
        "QuickSort (`introsort` = QuickSort + HeapSort fallback) has the best L1 cache locality for in-memory primitives; MergeSort and TimSort guarantee stability (preserving relative order of equal keys) and `O(N log N)` worst-case; Radix Sort beats `O(N log N)` on fixed-width 32/64-bit integers.",
        "Python's `list.sort()` and Rust's `slice::sort()` exploit natural ascending/descending runs (`TimSort` / `driftsort`), achieving `O(N)` time on already-partially-sorted real-world arrays.",
    ),
    (
        "Computer Science · Theory",
        "Computability, Turing Machines, The Halting Problem & P vs. NP",
        ["turing", "halting", "np", "npcomplete", "nphard", "computability", "decidable", "undecidable", "polynomial", "sat"],
        ["p vs np", "halting problem", "turing machine", "np complete", "np-complete", "np-hard", "boolean satisfiability"],
        "Computability theory defines what problems can be solved by any mechanical algorithm (Turing decidability), while complexity theory classifies decidable problems by how their time and space requirements scale (`P`, `NP`, `PSPACE`, `EXPTIME`).",
        "Alan Turing proved in 1936 that the Halting Problem (`does program P halt on input I?`) is undecidable via diagonalization: if a universal decider `H(P, I)` existed, constructing `D(P)` that loops if `H(P, P) == true` and halts if `H(P, P) == false` yields a contradiction on `D(D)`. In complexity theory, `P` is the class of decision problems solvable in polynomial time `O(N^k)` by a deterministic Turing machine, while `NP` is the class of problems whose solutions can be *verified* in polynomial time. By the Cook-Levin theorem, `3-SAT` is `NP-complete`: every problem in `NP` reduces to it in polynomial time.",
        "`P subseteq NP subseteq PSPACE subseteq EXPTIME`. If any single `NP-complete` problem (`3-SAT`, Traveling Salesperson, Clique, Graph 3-Coloring, Subset Sum) has a polynomial-time algorithm, then `P = NP`.",
        "When faced with an `NP-hard` problem in production, engineers avoid exponential brute force by using SAT/SMT solvers (`CDCL`), Binary Decision Diagrams (`BDDs`, verified in `R4-20`), integer linear programming, dynamic programming on bounded treewidth, or polynomial-time approximation algorithms.",
        "Modern public-key cryptography and zero-knowledge proofs rely on the asymmetry between hard forward search and fast verification.",
    ),
    (
        "Computer Science · Compilers",
        "Compilers, Lexing, Parsing, SSA Intermediate Representation & Code Generation",
        ["compiler", "compilers", "lexer", "parser", "ast", "ssa", "llvm", "bytecode", "jit", "register", "interpreter"],
        ["compiler design", "abstract syntax tree", "static single assignment", "register allocation", "just in time"],
        "A compiler translates high-level source code into optimized target machine code (or WebAssembly/bytecode) through a structured pipeline of frontend analysis, middle-end IR optimization, and backend code generation.",
        "1. **Frontend**: The Lexer tokenizes raw source bytes into lexemes using finite automata; the Parser (`LL(k)`, recursive descent, Pratt, or `LR(1)`) builds an Abstract Syntax Tree (`AST`); the Type Checker & Borrow/Scope Analyzer resolves symbols (`R4.Binder` de Bruijn indices) and enforces type invariants.\n2. **Middle-End (`SSA IR`)**: Lowering to Static Single Assignment (`SSA`) form ensures every variable is assigned exactly once (with `phi` nodes at control-flow joins), enabling dead-code elimination, constant folding, loop-invariant code motion, and inlining.\n3. **Backend**: Performs instruction selection, graph-coloring or linear-scan register allocation (`Chaitin-Briggs`), and target-specific scheduling.",
        "Graph-coloring register allocation maps interference graph vertices (live ranges) to `K` hardware registers; when the graph is not `K`-colorable, variables spill to the stack.",
        "Ahead-of-Time (`AOT`) compilers (Rust/`rustc`, C++/`clang`, Go) produce zero-warmup native binaries with predictable memory layouts, whereas Just-in-Time (`JIT`) compilers (V8, JVM, LuaJIT) profile hot paths at runtime for speculative deoptimization at the cost of higher memory overhead.",
        "In this repository (`lexlean-r4`), verified `.lex.tex` semantic ASTs are compiled deterministically into zero-float Rust and WebAssembly (`public/lexlean_r4_bg.wasm`).",
    ),
    (
        "Computer Science · Memory Management",
        "Garbage Collection vs. Rust Ownership, Borrowing & RAII",
        ["ownership", "borrow", "borrowing", "lifetime", "lifetimes", "raii", "garbage", "gc", "reference", "leak"],
        ["garbage collection", "borrow checker", "memory management", "mark and sweep", "reference counting"],
        "Programming languages manage heap memory safely through either compile-time ownership and deterministic destruction (`RAII` / Rust borrow checking) or runtime automatic reclamation (Reference Counting and Tracing Garbage Collection).",
        "In **Rust**, every value has a single owner, and the compiler enforces *Aliasing XOR Mutability* (`&T` shared immutable references OR one `&mut T` exclusive mutable reference) plus lifetime bounds (`'a`), freeing memory deterministically at scope exit with zero runtime pause. **Reference Counting** (`Arc`/`Rc`, Python, Swift) tracks strong reference counts and frees objects when `count == 0` (requiring weak references or cycle detectors for cyclic graphs). **Tracing Garbage Collectors** (Go, Java, V8) periodically trace reachable objects from stack/global roots via tri-color mark-and-sweep or generational copying collectors.",
        "Rust invariant: at any program point, `count(&mut T) == 1 => count(&T) == 0`. Generational hypothesis: most objects die young, so nursery minor GCs evacuate only the small surviving fraction.",
        "Rust Ownership/RAII provides zero GC pauses, minimal memory footprint, and fearless concurrency (data-race freedom at compile time) at the cost of a steeper learning curve. Tracing GCs simplify graph/cyclic data structures and rapid prototyping but incur tail-latency pauses and `1.5x-3x` heap headroom.",
        "In latency-critical systems (databases, kernels, WebAssembly engines like `lexlean-r4-wasm`), deterministic RAII avoids stop-the-world GC spikes entirely.",
    ),

    # ==================== AI, DEEP LEARNING & DATA SCIENCE ====================
    (
        "Artificial Intelligence · Deep Learning",
        "Neural Networks, Backpropagation, Gradient Descent & AdamW",
        ["backpropagation", "backprop", "gradient", "adam", "adamw", "sgd", "optimizer", "loss", "epoch", "overfitting", "regularization", "dropout"],
        ["gradient descent", "neural network", "learning rate", "loss function", "weight decay"],
        "Continuous neural networks learn parameterized functions `f_theta(x)` by minimizing a differentiable loss function `L(theta)` via reverse-mode automatic differentiation (backpropagation) and stochastic gradient-based optimizers.",
        "During the forward pass, inputs propagate through linear layers and non-linear activations (`GELU`, `SwiGLU`, `ReLU`) while intermediate activations are cached. During the backward pass, the chain rule computes exact partial derivatives `dL / dW_l` from the output layer back to the input layer. The **AdamW** optimizer maintains exponentially decaying first moments `m_t` (momentum) and second moments `v_t` (per-parameter adaptive variance) of the gradients, applying decoupled weight decay `theta <- (1 - lr * lambda) * theta - lr * m_hat_t / (sqrt(v_hat_t) + eps)`.",
        "Chain rule: `dL / dz_l = (W_{l+1}^T * dL / dz_{l+1}) odot sigma'(z_l)`. Bias-corrected moments: `m_hat_t = m_t / (1 - beta_1^t)`, `v_hat_t = v_t / (1 - beta_2^t)`.",
        "Standard SGD with momentum generalizes well on vision tasks with lower memory overhead (`1x` state), whereas AdamW is essential for Transformers and stiff loss landscapes despite requiring `3x-4x` parameter memory for optimizer states. In discrete integer models (`UOR-R4`), backpropagation is replaced by coupled coordinate sweeps (`R4.Learning.evaluateCoupledGateVector`) that never suffer from vanishing/exploding gradients.",
        "Combining cosine learning-rate warmup/decay, gradient clipping (`max_norm = 1.0`), Dropout, and decoupled weight decay prevents overfitting and stabilizes deep training.",
    ),
    (
        "Artificial Intelligence · Generative Models",
        "Diffusion Models, Score Matching & Denoising Generation",
        ["diffusion", "ddpm", "ddim", "denoising", "score", "latent", "unet", "dit", "guidance"],
        ["diffusion model", "stable diffusion", "score matching", "classifier free guidance", "flow matching"],
        "Diffusion and Flow-Matching models generate high-fidelity images, audio, video, and molecular structures by learning to reverse a gradual stochastic corruption process that maps structured data into Gaussian noise.",
        "1. **Forward Process**: Over `T` timesteps, clean data `x_0` is progressively mixed with Gaussian noise `epsilon ~ N(0, I)` according to variance schedule `alpha_bar_t`: `x_t = sqrt(alpha_bar_t) * x_0 + sqrt(1 - alpha_bar_t) * epsilon`.\n2. **Reverse Process**: A neural network `epsilon_theta(x_t, t, c)` (a U-Net or Diffusion Transformer `DiT` conditioned on text/class embedding `c`) predicts the noise `epsilon` (or velocity vector `v = x_1 - x_0` in Rectified Flow), which is equivalent to estimating the score function `grad_x log p_t(x)`. Iterative ODE/SDE solvers (`DDIM`, Euler, DPM-Solver) integrate from pure noise `x_T` back to a clean sample `x_0` in `10..50` steps.",
        "Variational training objective: `L_simple = E_{t, x_0, epsilon} [ || epsilon - epsilon_theta(sqrt(alpha_bar_t) x_0 + sqrt(1 - alpha_bar_t) epsilon, t, c) ||^2 ]`. Classifier-Free Guidance (`CFG`): `eps_guided = eps_uncond + w * (eps_cond - eps_uncond)`.",
        "Diffusion models achieve superior mode coverage and training stability compared to GANs (which suffer from mode collapse), at the cost of multi-step iterative sampling latency during inference (mitigated by Latent Diffusion VAEs and consistency distillation).",
        "Latent Diffusion runs the denoising trajectory inside a compressed `8x` lower-dimensional VAE latent space, reducing compute by `~64x` while preserving crisp perceptual detail.",
    ),
    (
        "Artificial Intelligence · Control & Alignment",
        "Reinforcement Learning (MDP, Q-Learning, PPO, RLHF & DPO)",
        ["reinforcement", "rl", "mdp", "qlearning", "ppo", "rlhf", "dpo", "policy", "reward", "bellman", "actor", "critic"],
        ["reinforcement learning", "markov decision process", "policy gradient", "proximal policy optimization", "direct preference optimization"],
        "Reinforcement Learning (`RL`) trains an agent to maximize cumulative discounted reward inside an environment modeled as a Markov Decision Process (`MDP`), and powers modern LLM post-training (`RLHF`, `DPO`, and reasoning RL).",
        "An MDP is defined by states `S`, actions `A`, transition probabilities `P(s' | s, a)`, rewards `R(s, a)`, and discount factor `gamma in [0, 1)`. **Value-based methods (`Q-Learning` / `DQN`)** learn the optimal action-value function `Q*(s, a)` via Temporal Difference (`TD`) updates. **Policy-gradient / Actor-Critic methods (`PPO`)** directly optimize stochastic policy `pi_theta(a | s)` using clipped probability ratios `r_t(theta) = pi_theta / pi_old` to prevent destructive policy updates. For language models, **DPO (Direct Preference Optimization)** derives a closed-form analytical mapping between reward functions and optimal policies, optimizing preference pairs `(y_win, y_lose)` directly without a separate reward model.",
        "Bellman Optimality: `Q*(s, a) = R(s, a) + gamma * E_{s'} [ max_{a'} Q*(s', a') ]`. PPO clipped objective: `L_CLIP = E [ min(r_t * A_t, clip(r_t, 1 - eps, 1 + eps) * A_t) ]`.",
        "Model-free RL (`PPO`, `SAC`) works without known environment physics but is sample-inefficient (`millions` of rollouts); Model-based RL (`MuZero`, World Models) and offline preference optimization (`DPO`) drastically improve sample efficiency.",
        "In code and math reasoning, verifiable-reward RL (`GRPO` / `PPO`) rewards the model whenever generated code passes unit tests or matches exact arithmetic proofs.",
    ),
    (
        "Artificial Intelligence · Machine Learning",
        "Classical Machine Learning (Decision Trees, Random Forests, XGBoost, SVM & PCA)",
        ["xgboost", "randomforest", "decisiontree", "svm", "kmeans", "pca", "clustering", "regression", "classification", "hyperplane"],
        ["random forest", "gradient boosting", "decision tree", "support vector machine", "principal component analysis", "k-means", "logistic regression"],
        "Classical machine learning algorithms remain the gold standard for tabular data, interpretable classification/regression, dimensionality reduction, and low-latency edge inference.",
        "- **Decision Trees & Ensembles**: Split features recursively to maximize Information Gain (Gini impurity or Shannon entropy). **Random Forests** bag independent trees over bootstrap samples to reduce variance; **Gradient Boosted Trees (`XGBoost` / `LightGBM`)** train trees sequentially where each tree fits the negative gradient (residuals) and Hessian of the previous ensemble's loss.\n- **Support Vector Machines (`SVM`)**: Find the maximum-margin hyperplane `w^T x + b = 0` separating classes, using kernel functions (`RBF`, polynomial) to project non-linear data into higher dimensions.\n- **PCA & K-Means**: PCA projects data onto orthogonal eigenvectors of the covariance matrix `X^T X`; K-Means iteratively alternates between assigning points to the nearest centroid and recomputing centroid means.",
        "Logistic Regression sigmoid: `P(y=1 | x) = 1 / (1 + exp(-(w^T x + b)))`. Gini Impurity: `G = 1 - sum_k p_k^2`. XGBoost optimal leaf weight: `w_j* = -G_j / (H_j + lambda)`.",
        "On structured tabular datasets with heterogeneous numerical/categorical columns and `< 1M` rows, Gradient Boosted Trees (`XGBoost`/`CatBoost`) consistently outperform deep neural networks in accuracy, training speed, and SHAP feature explainability.",
        "Fraud detection and credit scoring pipelines typically combine `XGBoost` for sub-millisecond tabular scoring with `PCA` / `Isolation Forest` for anomaly detection.",
    ),

    # ==================== SYSTEMS, OS, NETWORKING & DATABASES ====================
    (
        "Systems & Operating Systems",
        "Operating Systems: Processes, Threads, Scheduling & Virtual Memory",
        ["process", "processes", "thread", "threads", "kernel", "virtual", "paging", "tlb", "mmap", "syscall", "scheduler", "contextswitch"],
        ["operating system", "virtual memory", "page table", "context switch", "processes vs threads", "translation lookaside buffer"],
        "An Operating System (`OS`) kernel multiplexes hardware resources (CPU cores, physical RAM, NVMe storage, network NICs) across isolated user-space processes with hardware-enforced protection rings.",
        "- **Processes vs. Threads**: A process owns an isolated virtual address space (`CR3`/`TTBR0` page-table root) and file descriptor table; threads within a process share that address space (`heap`, code, globals) while maintaining separate stacks and CPU registers.\n- **Virtual Memory & Paging**: The hardware MMU translates 48-bit virtual addresses into physical `4 KB` (or `2 MB` hugepage) frames via a 4-level page-table walk cached in the CPU's Translation Lookaside Buffer (`TLB`).\n- **Copy-on-Write (`CoW`) & `mmap`**: `fork()` shares physical pages read-only until a write triggers a page fault that allocates a private copy; `mmap` maps files directly into virtual memory backed by the kernel page cache.",
        "Effective memory access time: `T_eff = p_TLB * T_cache + (1 - p_TLB) * (4 * T_mem + T_cache)`. Preemptive schedulers (`Linux CFS` / `EEVDF`) track virtual runtime (`vruntime`) in a red-black tree to guarantee fair CPU allocation.",
        "Processes provide strong fault and security isolation (a crash in one process never corrupts another's memory) at the cost of higher context-switch (`TLB` flush) and IPC overhead; threads have sub-microsecond context switches and zero-copy shared memory but require strict synchronization (`Mutex`, `RwLock`, atomics).",
        "High-performance engines use `mmap` with `2 MB` hugepages to eliminate `TLB` misses when scanning multi-megabyte lookup tables like `UORLUT01`.",
    ),
    (
        "Systems & Hardware Architecture",
        "CPU Architecture: Pipelining, Branch Prediction, SIMD/SWAR & Cache Coherency",
        ["cpu", "cache", "l1", "l2", "l3", "simd", "avx", "neon", "swar", "pipeline", "branch", "mesi", "numa"],
        ["cpu cache", "branch prediction", "cache coherency", "instruction pipeline", "out of order execution", "false sharing"],
        "Modern superscalar CPUs achieve high instruction throughput (`IPC`) through deep instruction pipelines, speculative branch prediction, out-of-order execution, vector `SIMD` lanes, and a multi-level (`L1/L2/L3`) coherent cache hierarchy.",
        "- **Memory Hierarchy**: `L1` data cache (`32-64 KB`, `~1 ns / 4 cycles`), `L2` (`512 KB - 2 MB`, `~4 ns`), `L3` shared LLC (`32-128 MB`, `~12 ns`), and main `DRAM` (`~60-80 ns`). Data moves in `64-byte` cache lines governed by the `MESI` (`Modified, Exclusive, Shared, Invalid`) coherency protocol.\n- **Branch Prediction & Out-of-Order (`OoO`)**: Tagged Geometric History Length (`TAGE`) predictors guess branch outcomes before condition evaluation; a branch misprediction flushes the pipeline (`~15-20` cycle penalty).\n- **SIMD & SWAR**: Single Instruction Multiple Data (`AVX-512`, `ARM NEON`, `Wasm SIMD128`) and SIMD-Within-A-Register (`SWAR` 64-bit bitwise operations) process `8..64` integer lanes in parallel.",
        "Amdahl's Law: `Speedup = 1 / ((1 - p) + p / S)`. Little's Law for memory bandwidth: `Concurrency = Bandwidth * Latency` (`64-byte` cache lines mean strided access wastes up to `93%` of bandwidth).",
        "Pointer-chasing data structures (`LinkedList`, pointer trees) stall the CPU on `60 ns` DRAM cache misses and defeat hardware prefetchers, whereas contiguous Struct-of-Arrays (`SoA`) layouts and branchless arithmetic (`select` / bitwise masks) saturate L1 cache and SIMD units.",
        "In `UOR-R4`, `/6` quaternion sign-code attention computes 64-dimensional similarity in a single 1-cycle hardware `POPCNT` (`(a ^ b).count_ones()`) instruction.",
    ),
    (
        "Systems & Concurrency",
        "Concurrency, Mutexes, Atomics, Lock-Free Data Structures & Deadlocks",
        ["concurrency", "parallelism", "mutex", "rwlock", "semaphore", "atomic", "cas", "deadlock", "race", "lockfree", "channel"],
        ["race condition", "deadlock prevention", "lock free", "compare and swap", "memory ordering", "message passing"],
        "Concurrency structures multiple independent tasks that make interleaved progress (e.g. async I/O), while parallelism executes multiple computations simultaneously across physical CPU/GPU cores.",
        "- **Synchronization Primitives**: A `Mutex` grants exclusive access to one thread; an `RwLock` allows concurrent readers or one exclusive writer; a `Semaphore` bounds concurrent access to `K` permits.\n- **Hardware Atomics (`CAS`)**: Lock-free algorithms coordinate state via hardware `Compare-And-Swap` (`compare_exchange`) and memory orderings (`Relaxed`, `Acquire`/`Release`, `SeqCst`) over `MESI` cache lines.\n- **Deadlocks**: A deadlock occurs if and only if all four **Coffman conditions** hold simultaneously: *Mutual Exclusion*, *Hold and Wait*, *No Preemption*, and *Circular Wait*.",
        "Breaking *Circular Wait* by enforcing a strict total ordering on lock acquisition (`always acquire Lock A before Lock B`) provably prevents deadlocks.",
        "Coarse-grained mutexes are simple to reason about but serialize threads under high contention; lock-free (`CAS`) ring buffers and message-passing channels (`CSP` / actor model) eliminate lock sleep/wake syscalls (`futex`) but require careful handling of the `ABA` problem and memory reclamation (`epoch-based reclamation`).",
        "A Single-Producer Single-Consumer (`SPSC`) ring buffer needs only `Acquire` loads and `Release` stores on `head` and `tail` indices—zero mutexes and zero `CAS` retries.",
    ),
    (
        "Systems & Networking",
        "Networking: TCP vs. UDP, QUIC, HTTP/1.1–HTTP/3, DNS & CDNs",
        ["tcp", "udp", "quic", "http", "http2", "http3", "dns", "cdn", "websocket", "grpc", "rest", "graphql", "packet", "socket"],
        ["tcp vs udp", "three way handshake", "3-way handshake", "how dns works", "rest vs graphql", "congestion control"],
        "The Internet protocol suite layers application protocols (`HTTP`, `DNS`, `gRPC`, `WebSockets`) over transport protocols (`TCP`, `UDP`, `QUIC`), network routing (`IPv4`/`IPv6`, `BGP`), and physical link frames.",
        "- **TCP**: Connection-oriented stream protocol that establishes sessions via a **3-Way Handshake (`SYN -> SYN-ACK -> ACK`)**, guarantees in-order delivery via sequence numbers and `ACK` retransmissions, enforces receiver flow control (sliding window), and regulates network load via congestion control (`CUBIC`, `BBR`).\n- **UDP & QUIC (`HTTP/3`)**: UDP sends stateless datagrams with zero handshake delay and no head-of-line (`HoL`) blocking. `QUIC` layers multiplexed streams, 0-RTT/1-RTT `TLS 1.3` encryption, and per-stream congestion control on top of UDP.\n- **DNS Resolution**: Translates domain names into IP addresses by querying browser/OS caches -> Recursive Resolver -> Root (` . `) -> TLD (`.com`/`.io`) -> Authoritative Name Server.",
        "Bandwidth-Delay Product (`BDP`): `Window_Size >= Bandwidth * RTT`. If a link is `1 Gbps` with `100 ms` RTT, saturating the pipe requires `12.5 MB` of in-flight unacknowledged data.",
        "Choose **TCP / HTTP/2 / REST** for general web APIs and reliable transactional transfers; choose **gRPC (Protobuf over HTTP/2)** for low-overhead internal microservice RPCs; choose **GraphQL** when frontend clients need flexible field selection across complex graphs; choose **UDP / QUIC / WebRTC** for real-time audio/video, gaming, and lossy networks where TCP's head-of-line blocking causes stutter.",
        "When a single packet is lost in `HTTP/2` over TCP, the OS kernel stalls *all* multiplexed streams until retransmission completes; `HTTP/3` (`QUIC` over UDP) stalls only the single affected stream.",
    ),
    (
        "Systems & Distributed Computing",
        "Distributed Systems: CAP Theorem, Consensus (Raft & Paxos), Sharding & Replication",
        ["distributed", "cap", "pacelc", "raft", "paxos", "consensus", "linearizability", "eventual", "quorum", "sharding", "replication", "idempotency"],
        ["cap theorem", "distributed system", "raft consensus", "eventual consistency", "strong consistency", "two phase commit"],
        "Distributed systems coordinate state and computation across independent networked machines that experience variable latency, clock drift, partial crashes, and network partitions.",
        "- **CAP & PACELC Theorems**: During a network **Partition (`P`)**, a distributed store must trade off **Consistency (`C`, linearizability)** against **Availability (`A`)**. Even without partitions (`E`), systems trade off **Latency (`L`)** against **Consistency (`C`)**.\n- **Consensus (`Raft` & `Paxos`)**: Replicate an ordered deterministic state-machine log across `2f + 1` nodes to tolerate `f` crash failures. Raft decomposes consensus into *Leader Election* (randomized election timeouts + term numbers), *Log Replication* (`AppendEntries` to a majority quorum `floor(N/2) + 1`), and *Safety* (only nodes with all committed entries can become leader).\n- **Replication & Sharding**: Partition data horizontally across shards via range partitioning or consistent hashing; use Quorum reads/writes (`R + W > N`) for tunable consistency.",
        "Majority Quorum intersection: any two subsets of size `floor(N/2) + 1` out of `N` nodes share at least one overlapping node (`(floor(N/2) + 1) * 2 > N`), guaranteeing every read quorum intersects the latest committed write quorum.",
        "`CP` systems (etcd, ZooKeeper, CockroachDB, Spanner) guarantee strict linearizable transactions at the cost of cross-node RTT latency and rejecting writes on minority partitions; `AP` systems (DynamoDB default, Cassandra) remain writable in all partitions using last-write-wins or vector clocks/CRDTs.",
        "To guarantee exactly-once business effects over at-least-once networks, pair exponential backoff with jitter alongside a client-generated **Idempotency Key** stored atomically with the database transaction.",
    ),
    (
        "Systems & Databases",
        "Databases: SQL vs. NoSQL, ACID Transactions, MVCC Isolation & Query Optimization",
        ["sql", "nosql", "acid", "mvcc", "transaction", "transactions", "isolation", "wal", "postgres", "postgresql", "sqlite", "mongodb", "redis", "join"],
        ["sql vs nosql", "acid transactions", "isolation levels", "multi version concurrency control", "write ahead log", "database indexing"],
        "Database management systems guarantee structured data querying, concurrent transaction isolation, and crash-proof durability across relational (`SQL`) and non-relational (`NoSQL`) models.",
        "- **ACID Guarantees**: **Atomicity** (all-or-nothing via undo logs), **Consistency** (foreign keys, unique/check constraints), **Isolation** (concurrent transactions execute without anomalous interference), and **Durability** (committed changes are fsynced to the Write-Ahead Log `WAL` before acknowledging).\n- **MVCC & Isolation Levels**: Multi-Version Concurrency Control tags rows with creation/deletion transaction IDs (`xmin`/`xmax`) so readers never block writers and writers never block readers. Standard SQL isolation levels progress from *Read Uncommitted* -> *Read Committed* -> *Repeatable Read (Snapshot Isolation)* -> *Serializable (`SSI`)*.\n- **Cost-Based Query Planner**: Parses SQL into relational algebra, estimates cardinality using table histograms, and selects between Nested Loop, Hash Join (`O(N + M)`), or Sort-Merge Join (`O(N log N + M log M)`).",
        "Write-Ahead Logging (`ARIES` protocol): log records must reach non-volatile storage *before* dirty data pages are flushed (`WAL` rule), enabling `Analysis -> Redo -> Undo` crash recovery.",
        "Relational `SQL` databases (`PostgreSQL`, `SQLite`, `MySQL`) are the best default for 90% of applications due to declarative joins, strict schemas, and ACID integrity. `NoSQL` engines shine for specific access patterns: `Redis` for sub-millisecond in-memory caching/queues, `DynamoDB`/`Cassandra` for massive single-key horizontal scale, and `ClickHouse`/`BigQuery` for columnar OLAP analytics.",
        "Adding a composite B-Tree index `CREATE INDEX ON orders (customer_id, created_at DESC)` turns a `500 ms` full sequential table scan (`Seq Scan`) into a `0.1 ms` `Index-Only Scan`.",
    ),
    (
        "Systems & Cloud Infrastructure",
        "Containers (Docker), Kubernetes Orchestration, Microservices & WebAssembly",
        ["docker", "kubernetes", "k8s", "container", "containers", "pod", "microservice", "microservices", "monolith", "wasm", "webassembly", "serverless"],
        ["docker vs vm", "virtual machine", "monolith vs microservices", "how docker works", "how kubernetes works"],
        "Modern cloud-native infrastructure packages applications into reproducible OCI containers (`Docker`), orchestrates them across clusters with `Kubernetes`, or executes them inside sub-millisecond `WebAssembly` sandboxes.",
        "- **Containers vs. Virtual Machines**: VMs virtualize hardware via a hypervisor (`KVM`/`Xen`), running a full guest OS kernel per instance (`hundreds of MBs`, seconds to boot). Containers share the host Linux kernel and isolate processes using **namespaces** (`pid`, `net`, `mnt`, `ipc`, `uts`, `user`) for visibility, **cgroups** for CPU/RAM quotas, and **OverlayFS** union mounts for immutable layered images.\n- **Kubernetes (`k8s`)**: Runs a declarative control loop (`apiserver`, `etcd`, `scheduler`, `kubelet`) that continuously reconciles actual cluster state toward desired manifests (`Deployment`, `Service`, `Ingress`, `HPA`).\n- **WebAssembly (`Wasm`)**: A portable binary instruction format with capability-based sandboxing and bounds-checked linear memory that boots in `< 1 ms` in browsers and edge runtimes.",
        "Reconciliation loop: `Delta = Desired_State(etcd) - Observed_State(kubelet)`. In WebAssembly, indirect calls are type-checked against a separate function table, preventing return-oriented programming (`ROP`) exploits inside linear memory.",
        "Start with a **Modular Monolith** to move fast with in-process function calls and single-database ACID transactions; extract **Microservices** only when independent team scaling, heterogeneous resource profiles (e.g. GPU vs CPU), or fault isolation justify network serialization and distributed tracing overhead.",
        "This entire UOR-R4 platform compiles its verified LexLean/Rust engine into a single `~190 KB` WebAssembly module (`public/lexlean_r4_bg.wasm`) that runs offline in the browser with zero server dependencies.",
    ),

    # ==================== PROGRAMMING LANGUAGES & CRYPTOGRAPHY ====================
    (
        "Programming Languages · Comparison",
        "Rust vs. Go vs. Python vs. TypeScript vs. C++",
        ["rust", "golang", "python", "typescript", "javascript", "cpp", "c++", "language", "languages"],
        ["rust vs go", "python vs typescript", "rust vs c++", "python vs rust", "typescript vs javascript"],
        "Each modern programming language occupies a distinct point on the engineering trade-off frontier between raw hardware control, memory safety, concurrency ergonomics, and developer iteration velocity.",
        "- **Rust**: Compiled to native code via LLVM; guarantees memory and thread safety at compile time via Ownership & Borrow Checking without a garbage collector (`0 ms` GC pauses). Ideal for kernels, databases, Wasm, crypto, and high-throughput infrastructure.\n- **Go (Golang)**: Compiles in seconds to a static binary; uses a concurrent low-latency garbage collector and lightweight `M:N` goroutines (`2 KB` stacks) with typed channels. Ideal for cloud-native microservices, CLIs, and networked APIs.\n- **Python**: Dynamically typed (with optional `mypy`/`pyright` type hints); powered by a vast C/Rust/CUDA ecosystem (`PyTorch`, `NumPy`, `Pandas`, `FastAPI`). Ideal for AI/ML, data science, automation, and rapid backend prototyping.\n- **TypeScript**: Adds a sound structural type system, discriminated unions, and IDE refactoring over JavaScript (`V8` event loop). The universal standard for web frontends, full-stack apps, and Node/Bun/Deno services.\n- **C++**: Zero-overhead hardware abstraction (`RAII`, templates, `constexpr`, SIMD) with manual/smart-pointer memory management, dominant in game engines, CUDA GPU kernels, and embedded systems.",
        "Latency & Throughput Spectrum: `Rust / C++ (native, zero-GC) > Go (native, concurrent GC) > TypeScript/V8 (JIT) > CPython (bytecode interpreter)`.",
        "Use **Rust** when correctness, predictable tail latency, memory footprint, or WebAssembly portability are paramount; use **Go** for high-velocity team backend services; use **Python** for ML/data pipelines and scripting; use **TypeScript** for interactive UIs and web platforms.",
        "Many production systems combine **TypeScript** for the browser UI, **Python/CUDA** for offline GPU training, and **Rust/WebAssembly** for the verified inference kernel—just like this repository.",
    ),
    (
        "Cybersecurity & Cryptography",
        "Cryptography: AES-GCM, RSA, Elliptic Curves (ECC), SHA-256, BLAKE3 & Zero-Knowledge Proofs",
        ["cryptography", "encryption", "aes", "rsa", "ecc", "ed25519", "diffie", "sha256", "blake3", "merkle", "zkp", "zksnark", "tls", "jwt", "oauth"],
        ["public key cryptography", "symmetric vs asymmetric", "zero knowledge proof", "elliptic curve", "how tls works", "merkle tree"],
        "Modern cryptography secures communication and state integrity through four foundational primitives: Symmetric Authenticated Encryption (`AEAD`), Asymmetric Public-Key Key Exchange/Signatures, Cryptographic Hash/Merkle Functions, and Zero-Knowledge Proofs (`ZKPs`).",
        "1. **Symmetric AEAD (`AES-256-GCM`, `ChaCha20-Poly1305`)**: Encrypts bulk data and attaches a `128-bit` authentication tag using a shared secret key and unique nonce.\n2. **Asymmetric / Public-Key (`RSA`, `X25519`, `Ed25519`, Post-Quantum `ML-KEM`)**: `TLS 1.3` uses Ephemeral Elliptic-Curve Diffie-Hellman (`ECDHE` over Curve25519) so two parties derive a shared session secret over an untrusted channel with **Forward Secrecy** (compromising a long-term key later cannot decrypt past sessions).\n3. **Cryptographic Hashes (`SHA-256`, `BLAKE3`)**: Map arbitrary inputs to fixed `256-bit` digests with preimage, second-preimage, and collision resistance (`2^128` birthday bound). `BLAKE3` structures every input as a binary Merkle tree of `1 KB` chunks for unlimited SIMD/multicore parallelism.\n4. **Zero-Knowledge Proofs (`ZK-SNARKs` / `ZK-STARKs`)**: Allow a Prover to convince a Verifier that they know a secret witness `w` satisfying `C(x, w) = 1` without revealing any information about `w`.",
        "Diffie-Hellman over group `G`: Alice sends `g^a`, Bob sends `g^b`; both compute shared secret `(g^a)^b = (g^b)^a = g^(ab)`, while an eavesdropper faces the Discrete Logarithm problem.",
        "Asymmetric operations (`ECC`/`RSA`) are `~1,000x` slower than symmetric `AES-GCM`/`ChaCha20`, so protocols like `TLS 1.3` and `SSH` use asymmetric crypto only during the handshake to exchange an ephemeral key, then switch to symmetric AEAD for all payload frames.",
        "In `lexlean-r4`, both `SHA-256` (`R4-14`) and `BLAKE3` (`R4-15`) are formally proved in zero-axiom Lean 4 and verify every state receipt in `public/provenance.json`.",
    ),
]


GLM_CONCEPTS_PART2 = [
    # ==================== MATHEMATICS, LOGIC, PROBABILITY & GAME THEORY ====================
    (
        "Mathematics · Analysis",
        "Calculus, Derivatives, Integrals, Taylor Series & Differential Equations",
        ["calculus", "derivative", "derivatives", "integral", "integrals", "taylor", "ode", "pde", "limit", "chainrule", "gradient", "laplace", "fourier"],
        ["fundamental theorem of calculus", "taylor series", "differential equation", "fourier transform", "integration by parts"],
        "Calculus and mathematical analysis study continuous rates of change (differential calculus), accumulation of quantities (integral calculus), infinite series approximations (Taylor/Fourier), and dynamical laws expressed as Ordinary and Partial Differential Equations (`ODEs` & `PDEs`).",
        "- **Derivatives & Chain Rule**: The derivative `f'(x) = lim_{h->0} (f(x+h) - f(x))/h` gives the instantaneous slope; for composite functions `(f o g)'(x) = f'(g(x)) * g'(x)` (the foundation of backpropagation).\n- **Fundamental Theorem of Calculus**: Differentiation and integration are exact inverses: `integral_a^b f'(x) dx = f(b) - f(a)`.\n- **Taylor Series & Fourier Transform**: Taylor series expand smooth functions around `x = a` as `sum_{n=0}^inf (f^(n)(a) / n!) (x - a)^n`; the Fourier Transform decomposes any time/space signal `f(t)` into orthogonal complex sinusoids `F(omega) = integral f(t) e^(-i omega t) dt` (`FFT` computes discrete Fourier transforms in `O(N log N)`).\n- **Differential Equations**: Govern physical evolution (`dx/dt = f(x, t)`), solved analytically via separation/Laplace transforms or numerically via 4th-order Runge-Kutta (`RK4`).",
        "Euler's Identity: `e^(i * pi) + 1 = 0`. Convolution Theorem: `F{f * g} = F{f} . F{g}` (multiplying in the frequency domain equals convolving in the time domain).",
        "Analytical closed-form solutions exist only for linear or integrable systems; stiff or non-linear PDEs (like Navier-Stokes fluid flow or N-body gravity) require symplectic integrators, finite-element meshes, or spectral FFT methods.",
        "Audio compression (`MP3`/`Opus`), MRI image reconstruction, and fast polynomial multiplication all rely on the `O(N log N)` Fast Fourier Transform (`FFT`).",
    ),
    (
        "Mathematics · Algebra & Geometry",
        "Linear Algebra, Eigenvalues, Singular Value Decomposition (SVD), Quaternions & Clifford Algebras",
        ["linear", "matrix", "matrices", "eigenvalue", "eigenvalues", "eigenvector", "svd", "quaternion", "quaternions", "clifford", "e8", "icosian", "lattice", "determinant"],
        ["linear algebra", "singular value decomposition", "eigenvalues and eigenvectors", "clifford algebra", "e8 lattice", "hamilton product"],
        "Linear and geometric algebras model high-dimensional vector spaces, orthogonal transformations, spectral decompositions (`Eigenvalues` & `SVD`), and hypercomplex spinor rotations (`Quaternions`, `Clifford Algebras`, and the `$E_8$` / `Icosian` lattices).",
        "- **Eigenvalues (`A v = lambda v`)**: Eigenvectors `v` are invariant directions under linear map `A`, scaled by eigenvalue `lambda`. The matrix trace equals `sum lambda_i` and the determinant (signed volume scaling factor) equals `prod lambda_i`.\n- **Singular Value Decomposition (`A = U * Sigma * V^T`)**: Factorizes any `m x n` matrix into orthogonal left/right singular vectors `U, V` and non-negative singular values `Sigma`, yielding the optimal rank-`k` approximation (`Eckart-Young theorem`).\n- **Quaternions (`H`) & Clifford Algebra (`Cl(0,6)`)**: Unit quaternions `q = w + xi + yj + zk` (`i^2 = j^2 = k^2 = ijk = -1`) represent 3D/4D rotations without gimbal lock. The `120` unit icosians form the binary icosahedral group inside `Z[phi]` and project onto the `E8` root lattice.",
        "Spectral Theorem: Every real symmetric matrix `A = A^T` is orthogonally diagonalizable (`A = Q Lambda Q^T`) with real eigenvalues. In `UOR-R4`, `R4.SpiralCoreFbs` (`R4-24`) snaps 4-lane `Q12` blocks onto the 17-root Icosian `Z[phi]` table.",
        "Dense matrix inversion `A^-1` is `O(N^3)` and numerically unstable on ill-conditioned matrices (`high condition number kappa = sigma_max / sigma_min`); `QR`, `Cholesky`, `SVD`, or discrete group actions (`Cl(0,6)`) avoid catastrophic cancellation.",
        "Low-Rank Adaptation (`LoRA`) in ML and image compression both truncate small singular values in `SVD` (`W + B * A` where `rank r << d`).",
    ),
    (
        "Mathematics · Foundations & Logic",
        "Formal Logic, Gödel's Incompleteness Theorems, Category Theory, Monads & Lean 4",
        ["godel", "incompleteness", "category", "monad", "monads", "functor", "lean", "lean4", "coq", "curry", "howard", "axiom", "theorem", "proof", "bdd"],
        ["godel incompleteness", "category theory", "curry howard", "dependent types", "formal verification", "binary decision diagram"],
        "Mathematical logic and type theory provide the rigorous foundations for proof assistants (`Lean 4`, `LexLean`), program verification, and algebraic composition (`Category Theory`).",
        "- **Gödel's First & Second Incompleteness Theorems**: Any consistent formal system capable of expressing Peano arithmetic contains true statements that cannot be proved within the system (constructed via Gödel numbering and self-referential diagonalization), and cannot prove its own consistency.\n- **Curry-Howard Isomorphism (`Propositions as Types`)**: A logical proposition is a *Type*, and a proof of that proposition is a terminating *Program (Term)* inhabiting that type (`Implication A -> B` is a function type; `Conjunction A /\\ B` is a product pair; `Universal quantification forall x, P(x)` is a dependent function `(x : A) -> P x`).\n- **Category Theory & Monads**: A Category consists of objects and associative morphisms with identity. A **Monad** (`(M, pure, bind >>=)`) is a functor equipped with unit (`return`/`pure`) and join (`flatMap`/`>>=`) satisfying left-identity, right-identity, and associativity—allowing pure functional languages (`Lean 4`, `Haskell`, `Rust Option/Result`) to sequence state, I/O, and error handling deterministically.",
        "Monad Associativity Law: `(m >>= f) >>= g  ==  m >>= (\\x -> f x >>= g)`.",
        "Testing can only show the *presence* of bugs on sampled inputs, whereas formal verification in **Lean 4** checks every possible input path mathematically at compile time (with zero runtime overhead once erased).",
        "In this repository (`lexlean-r4`), all 27 `R4-01..R4-27` capabilities are proved in zero-axiom Lean 4.32.1 (`#print axioms` confirms zero `sorryAx` and zero custom axioms).",
    ),
    (
        "Probability, Statistics & Information Theory",
        "Probability, Bayes' Theorem, Central Limit Theorem, Hypothesis Testing & Shannon Entropy",
        ["probability", "bayes", "bayesian", "gaussian", "clt", "pvalue", "hypothesis", "confidence", "variance", "shannon", "kldivergence", "markov", "montecarlo"],
        ["bayes theorem", "central limit theorem", "hypothesis testing", "shannon entropy", "information theory", "markov chain", "monte carlo"],
        "Probability theory, statistical inference, and Claude Shannon's information theory quantify uncertainty, belief updating under evidence, and the fundamental limits of data compression and transmission.",
        "- **Bayes' Theorem**: Updates prior belief `P(H)` given observed evidence `E` via `P(H | E) = P(E | H) * P(H) / P(E)`.\n- **Central Limit Theorem (`CLT`)**: The sample mean of `N` independent, identically distributed random variables with finite variance `sigma^2` converges in distribution to a Gaussian `N(mu, sigma^2 / N)` regardless of the underlying distribution's shape.\n- **Statistical Testing**: A `p-value` is the probability of observing a test statistic at least as extreme as the sample *assuming the null hypothesis `H_0` is true* (it is NOT `P(H_0 is true | data)`!).\n- **Shannon Information Theory**: The entropy `H(X) = -sum p(x) log2 p(x)` measures the minimum average bits needed to encode outcomes of `X`. Cross-entropy `H(P, Q) = H(P) + D_KL(P || Q)` measures the extra bits wasted when coding distribution `P` using model `Q`.",
        "Bayes' Rule: `Posterior = (Likelihood * Prior) / Marginal_Evidence`. Shannon Channel Capacity: `C = B * log2(1 + S / N)` bits/sec.",
        "Frequentist inference (`p-values`, confidence intervals) avoids choosing subjective priors and controls long-run Type I/II error rates; Bayesian inference incorporates domain priors naturally and handles small-sample sequential updating cleanly.",
        "When testing a rare condition (`0.1%` prevalence) with a `99%` accurate test, Bayes' theorem reveals that a single positive result has only a `~9%` true-positive probability (`base-rate fallacy`) until confirmed by a second independent test.",
    ),
    (
        "Mathematics · Game Theory & Decision Science",
        "Game Theory: Nash Equilibrium, Prisoner's Dilemma, Pareto Optimality & Mechanism Design",
        ["gametheory", "nash", "prisoner", "pareto", "minimax", "auction", "vickrey", "schelling", "cooperate", "defect"],
        ["game theory", "nash equilibrium", "prisoner's dilemma", "prisoners dilemma", "pareto optimal", "zero sum"],
        "Game theory analyzes strategic interactions where each participant's payoff depends on the choices of all other rational agents.",
        "- **Nash Equilibrium**: A strategy profile `(s_1*, ..., s_n*)` where no player can increase their expected payoff by unilaterally changing their strategy while others keep theirs fixed (John Nash proved every finite game has at least one mixed-strategy Nash equilibrium).\n- **The Prisoner's Dilemma**: In a single-shot game, *Defect* strictly dominates *Cooperate* for each individual player, trapping both in a mutually worse `(Defect, Defect)` Nash equilibrium rather than the **Pareto-optimal** `(Cooperate, Cooperate)` outcome. In **Iterated Prisoner's Dilemma** (Robert Axelrod's tournaments), *Tit-for-Tat* (start cooperative, mirror partner's previous move, forgive occasionally) sustains cooperation whenever the discount factor `delta` (shadow of the future) is high enough.\n- **Mechanism Design (Inverse Game Theory)**: Designs auction/voting rules (such as the **Vickrey Second-Price Auction**) where truthful bidding is a dominant strategy.",
        "Cooperation condition in infinitely repeated Prisoner's Dilemma: `delta >= (T - R) / (T - P)` where `T > R > P > S` (`Temptation > Reward > Punishment > Sucker`).",
        "Single-shot anonymous interactions incentivize defection (tragedy of the commons, price wars), whereas repeated interactions, transparent reputation ledgers, and cryptographic commitments shift the equilibrium to cooperation.",
        "In online ad auctions, pricing networks, and consensus protocols, mechanism design ensures self-interested nodes maximize their own payoff by following the honest protocol.",
    ),

    # ==================== PHYSICS, QUANTUM MECHANICS, RELATIVITY & ASTRONOMY ====================
    (
        "Physics · Classical, Fluid & Thermodynamics",
        "Classical Mechanics, Thermodynamics, Entropy & Fluid Dynamics (Bernoulli & Navier-Stokes)",
        ["newton", "lagrangian", "hamiltonian", "thermodynamics", "entropy", "carnot", "gibbs", "bernoulli", "navier", "aerodynamics", "lift", "airplane"],
        ["laws of thermodynamics", "second law of thermodynamics", "how airplanes fly", "bernoulli principle", "navier stokes", "conservation of energy"],
        "Classical mechanics, thermodynamics, and fluid dynamics govern macroscopic motion, energy conversion, heat engines, entropy, and aerodynamic flight.",
        "- **Newtonian & Hamiltonian Mechanics**: `F = dp/dt = m a`. By **Noether's Theorem**, every continuous symmetry of the action integral corresponds to a conserved quantity: time-translation symmetry -> *Conservation of Energy*; space-translation -> *Conservation of Linear Momentum*; rotational symmetry -> *Conservation of Angular Momentum*.\n- **Thermodynamics**:\n  1. **1st Law (`dU = delta Q - delta W`)**: Total energy is conserved.\n  2. **2nd Law (`dS >= 0`, `S = k_B ln Omega`)**: Isolated systems evolve toward macroscopic states with higher microstate multiplicity (`Omega`); maximum heat engine efficiency is the **Carnot limit `eta = 1 - T_cold / T_hot`**.\n- **Fluid Dynamics & Flight**: Airplanes generate lift through both **Newton's 3rd Law** (angled airfoil deflects a massiveair stream downward, pushing the wing upward) and **Bernoulli's Principle** (`P + 1/2 rho v^2 + rho g h = const`: faster airflow curving over the upper camber creates a low-pressure region above the wing).",
        "Boltzmann Entropy: `S = k_B ln(Omega)`. Gibbs Free Energy: `Delta G = Delta H - T Delta S` (`Delta G < 0` for spontaneous processes). Carnot Efficiency: `eta_max = 1 - T_C / T_H` (in Kelvin).",
        "Ideal inviscid equations (Euler/Bernoulli) accurately predict pressure distributions in streamlined flow, while full **Navier-Stokes equations** include viscosity and turbulent boundary layers (`Reynolds number Re = rho v L / mu`).",
        "A power plant operating between `T_hot = 900 K` and `T_cold = 300 K` has a theoretical maximum Carnot efficiency of `1 - 300/900 = 66.7%`.",
    ),
    (
        "Physics · Electromagnetism, Optics & Solid-State",
        "Electromagnetism (Maxwell's Equations), Rayleigh Scattering, Semiconductors & Batteries",
        ["maxwell", "electromagnetism", "magnetic", "rayleigh", "sky", "sunset", "rainbow", "semiconductor", "transistor", "mosfet", "battery", "lithium", "superconductor"],
        ["maxwell equations", "why is the sky blue", "how transistors work", "lithium ion battery", "how batteries work", "rayleigh scattering"],
        "Maxwell's four equations unify electricity, magnetism, and optics, explaining everything from why the sky is blue (`Rayleigh scattering`) to how `MOSFET` silicon transistors and `Lithium-ion` batteries power modern civilization.",
        "- **Maxwell's Equations & Light**: Time-varying electric fields generate magnetic fields and vice versa, propagating in vacuum as transverse electromagnetic waves at speed `c = 1 / sqrt(mu_0 eps_0) = 299,792,458 m/s`.\n- **Why the Sky is Blue & Sunsets are Red (`Rayleigh Scattering`)**: Atmospheric `N2` and `O2` molecules are much smaller than visible wavelengths (`400-700 nm`). Scattering intensity scales inversely with the fourth power of wavelength (`I propto 1 / lambda^4`), so short-wavelength blue light (`450 nm`) scatters `~5.5x` more across the sky than red light (`650 nm`). At sunset, sunlight travels through `~10x` more atmosphere, scattering away blue/green and leaving warm red/orange hues.\n- **Semiconductors & Transistors (`MOSFET`)**: Doping silicon (`1.12 eV` bandgap) with phosphorus (`n-type`) or boron (`p-type`) creates `p-n` junctions; applying voltage to a MOSFET gate forms an inversion channel, switching current in picoseconds.\n- **Lithium-Ion Batteries**: During discharge, `Li+` ions de-intercalate from the graphite anode, migrate through the electrolyte separator, and intercalate into the cathode (`NMC` or `LFP`) while electrons flow through the external circuit.",
        "Maxwell-Ampère Law: `curl B = mu_0 J + mu_0 eps_0 dE/dt`. Rayleigh Scattering cross-section: `sigma_s propto omega^4 ~ 1 / lambda^4`.",
        "`LFP` (Lithium Iron Phosphate) batteries offer longer cycle life (`3,000+` cycles) and thermal runaway safety, while `NMC` (Nickel Manganese Cobalt) batteries offer higher gravimetric energy density (`Wh/kg`).",
        "Every modern CPU/GPU packs tens of billions of `3 nm - 5 nm` FinFET/GAAFET CMOS transistors switching via electric-field gate capacitance.",
    ),
    (
        "Physics · Relativity, Quantum Mechanics & Nuclear Energy",
        "Relativity, Quantum Mechanics, Entanglement, The Standard Model & Nuclear Fusion/Fission",
        ["quantum", "relativity", "einstein", "entanglement", "superposition", "schrodinger", "heisenberg", "uncertainty", "qubit", "fission", "fusion", "higgs", "quark"],
        ["general relativity", "special relativity", "quantum mechanics", "quantum entanglement", "quantum computing", "nuclear fusion", "nuclear fission", "standard model"],
        "Modern physics rests on two pillars: **Einstein's General Relativity** (smooth spacetime geometry governing gravity and cosmology) and **Quantum Mechanics / Quantum Field Theory** (discrete probabilistic wavefunctions governing matter and subatomic forces).",
        "- **Special & General Relativity**: The speed of light `c` is constant in all inertial frames, yielding time dilation (`dt' = gamma dt`), length contraction, mass-energy equivalence (`E^2 = (pc)^2 + (m c^2)^2`), and gravitational spacetime curvature (`G_mu_nu = 8 pi G / c^4 T_mu_nu`) confirmed by GPS atomic clocks, gravitational lensing, and LIGO gravitational waves.\n- **Quantum Mechanics & Entanglement**: Systems evolve unitarily via the **Schrödinger Equation** (`i hbar d|psi>/dt = H |psi>`) in linear superposition `alpha|0> + beta|1>` until measurement projects onto an eigenstate with probability `|alpha|^2` (**Born Rule**). **Heisenberg's Uncertainty Principle** (`Delta x * Delta p >= hbar / 2`) stems from non-commuting operators `[x, p] = i hbar`. **Quantum Entanglement** creates non-separable multi-particle states (`(|00> + |11>)/sqrt(2)`) that violate **Bell's Inequality** (`|S| = 2*sqrt(2) > 2`) while respecting relativistic causality (no faster-than-light signaling).\n- **Nuclear Fission vs. Fusion**: Iron-56 (`56Fe`) has the highest nuclear binding energy per nucleon (`~8.8 MeV`). **Fission** splits heavy nuclei (`235U`, `239Pu`) via neutron chain reactions; **Fusion** fuses light nuclei (Deuterium + Tritium -> `4He + n + 17.6 MeV`) at `>100 million K` plasma temperatures (`Lawson criterion`).",
        "Schrödinger: `i hbar d|psi>/dt = H |psi>`. Einstein Field Equations: `R_uv - 1/2 R g_uv + Lambda g_uv = (8 pi G / c^4) T_uv`. Mass defect energy: `Delta E = Delta m * c^2`.",
        "Quantum computers (`Qubits`) achieve exponential speedups on structured algebraic problems (Shor's `O((log N)^3)` factoring, quantum chemistry simulation) and quadratic speedups on unstructured search (Grover's `O(sqrt(N))`), but require surface-code quantum error correction against environmental decoherence.",
        "GPS satellites must apply both Special Relativity (`-7 microseconds/day` from orbital speed) and General Relativity (`+45 microseconds/day` from weaker gravity aloft); without Einstein's `+38 us/day` net correction, GPS positions would drift by `~11 km` every single day!",
    ),
    (
        "Astronomy, Astrophysics & Cosmology",
        "Astrophysics: Stars, Black Holes, Dark Matter, Exoplanets, Tides & Seasons",
        ["astronomy", "solar", "planet", "planets", "star", "supernova", "neutron", "blackhole", "hawking", "bigbang", "darkmatter", "darkenergy", "exoplanet", "europa", "tides", "seasons"],
        ["black hole", "dark matter", "dark energy", "big bang", "solar system", "what causes the seasons", "what causes tides", "hawking radiation"],
        "Astrophysics and cosmology trace the evolution of the universe from the **Big Bang (`13.8 billion years ago`)** to galaxy formation, stellar nucleosynthesis, black holes, and planetary climates.",
        "- **Earth's Seasons & Ocean Tides**: Earth's seasons are caused by its **`23.44°` axial tilt** relative to its orbital plane (changing solar incidence angle and daylight duration)—*not* by orbital distance! Ocean tides are caused by the **gravitational gradient (differential pull)** of the Moon (and Sun) across Earth's diameter, creating two tidal bulges.\n- **Stellar Lifecycle & Nucleosynthesis**: Stars fuse `H -> He` on the main sequence; low-mass stars (`< 8 M_sun`) become Red Giants and shed planetary nebulae leaving **White Dwarfs** (supported by electron degeneracy up to the `1.44 M_sun` Chandrasekhar limit); massive stars fuse up to Iron (`56Fe`) and explode as core-collapse **Supernovae**, forging heavy elements (`Au`, `Pt`, `U`) and leaving **Neutron Stars** or **Black Holes**.\n- **Black Holes & Cosmology**: Inside the Schwarzschild radius `R_s = 2GM / c^2`, escape velocity exceeds `c`; quantum vacuum fluctuations near the horizon emit **Hawking Radiation** (`T_H = hbar c^3 / (8 pi G M k_B)`). On cosmic scales, universe energy density comprises `~68% Dark Energy` (accelerating expansion), `~27% Dark Matter` (inferred from flat galactic rotation curves and gravitational lensing), and `~5% Baryonic Matter`.",
        "Kepler's Third Law: `T^2 = (4 pi^2 / G M) a^3`. Hubble's Law: `v = H_0 * d`. Schwarzschild Radius: `R_s = 2 G M / c^2` (`~3 km` per solar mass).",
        "Transit photometry (`Kepler`, `TESS`, `JWST`) detects exoplanets by measuring periodic stellar dimming `(R_p / R_star)^2` and analyzes atmospheric transmission spectra for biosignature gases (`H2O`, `CO2`, `CH4`, `O2`).",
        "The calcium in our bones and the iron in our blood were literally synthesized inside ancient stars and supernovae billions of years before the Solar System formed.",
    ),

    # ==================== CHEMISTRY, BIOLOGY, GENETICS, NEUROSCIENCE & MEDICINE ====================
    (
        "Chemistry & Materials Science",
        "Chemistry: Periodic Table, Chemical Bonding, Acids/Bases, Catalysis & Organic Reactions",
        ["chemistry", "periodic", "atom", "electron", "orbital", "covalent", "ionic", "hydrogenbond", "ph", "acid", "base", "catalyst", "redox", "polymer", "maillard"],
        ["periodic table", "chemical bonding", "covalent vs ionic", "acids and bases", "le chatelier", "maillard reaction", "organic chemistry"],
        "Chemistry explains how atomic electron configurations (`s, p, d, f` quantum orbitals) govern chemical bonding, molecular geometry, reaction kinetics, catalysis, and material properties.",
        "- **Chemical Bonding**: **Covalent bonds** share valence electron pairs (`sigma` and `pi` molecular orbitals); **Ionic bonds** transfer electrons between low-ionization metals and high-electronegativity non-metals (`NaCl` lattice); **Metallic bonds** delocalize electrons in a conductive sea; **Hydrogen bonds** (`O-H...O`, `N-H...N`) give water its anomalously high boiling point, surface tension, and ice buoyancy while stabilizing `DNA` base pairs and protein `alpha-helices`.\n- **Equilibrium, Catalysis & pH**: By **Le Chatelier's Principle**, a system at equilibrium shifts to counteract applied changes in concentration, temperature, or pressure. **Catalysts (and Enzymes)** accelerate reactions by stabilizing the transition state and lowering the activation energy `E_a` without altering `Delta G`.\n- **Acids, Bases & Redox**: `pH = -log10[H3O+]` (`pH 7` neutral; each unit is a `10x` change in proton activity). **Redox** reactions transfer electrons (`OIL RIG`: Oxidation Is Loss, Reduction Is Gain).",
        "Arrhenius Rate Law: `k = A * exp(-E_a / (R * T))` (a small drop in activation energy `E_a` increases reaction speed exponentially). Henderson-Hasselbalch: `pH = pK_a + log10([A-] / [HA])`.",
        "Thermodynamics (`Delta G`) tells you whether a reaction *can* happen spontaneously (e.g. diamond converting to graphite has `Delta G < 0`), while Kinetics (`E_a`) tells you *how fast* it happens (diamond's enormous activation barrier makes it kinetically stable for billions of years).",
        "In cooking, the **Maillard Reaction** (`140-165 °C`) reacts nucleophilic amino groups of proteins with carbonyl groups of reducing sugars to create hundreds of savory aroma and golden-brown crust compounds.",
    ),
    (
        "Biology, Genetics & Biotechnology",
        "Biology & Genetics: Photosynthesis, Cellular Respiration, DNA/RNA, CRISPR & Evolution",
        ["biology", "photosynthesis", "respiration", "mitochondria", "atp", "krebs", "dna", "rna", "mrna", "ribosome", "protein", "crispr", "evolution", "mitosis", "meiosis"],
        ["cellular respiration", "central dogma", "dna vs rna", "natural selection", "crispr cas9", "mitosis vs meiosis", "how photosynthesis works"],
        "Life operates as a self-replicating thermodynamic system that harvests solar/chemical energy to maintain low internal entropy (`ATP` metabolism) and propagates digital genetic information via the Central Dogma (`DNA -> RNA -> Protein`) and Darwinian evolution.",
        "- **Bioenergetics**: **Photosynthesis** in chloroplasts uses light to split water (`2 H2O -> O2 + 4H+ + 4e-`), generating `ATP` and `NADPH` to fix `CO2` into glucose via the Calvin Cycle (`6 CO2 + 6 H2O -> C6H12O6 + 6 O2`). **Cellular Respiration** oxidizes glucose through *Glycolysis* (cytosol) -> *Krebs/Citric Acid Cycle* (mitochondrial matrix) -> *Oxidative Phosphorylation* (inner mitochondrial membrane proton gradient driving rotary `ATP Synthase` to yield `~30-32 ATP` per glucose).\n- **Genetics & Central Dogma**: Double-stranded `DNA` (`A-T`, `G-C` deoxyribose backbone) is transcribed by RNA Polymerase into single-stranded `mRNA` (`A-U`, `G-C`), which ribosomes translate 3 bases (1 codon) at a time via `tRNA` anticodons into 20 amino acids that fold into 3D proteins.\n- **CRISPR-Cas9**: Uses a synthetic single-guide RNA (`sgRNA`) to direct the `Cas9` endonuclease to a matching 20-nucleotide genomic target adjacent to a `PAM` (`NGG`) motif, making a precise double-strand cut for gene knockout, base editing, or prime editing.\n- **Evolution**: Driven by *Mutation*, *Natural Selection* (differential reproductive fitness), *Genetic Drift*, and *Gene Flow*.",
        "Hardy-Weinberg Equilibrium: allele frequencies `p + q = 1` and genotype frequencies `p^2 + 2pq + q^2 = 1` remain constant in the absence of selection, mutation, migration, and drift.",
        "**Mitosis** produces `2` genetically identical diploid (`2n`) somatic cells for growth and tissue repair; **Meiosis** performs homologous recombination (crossing-over) and two divisions to produce `4` genetically unique haploid (`1n`) gametes.",
        "Every day, an average human body recycles roughly its entire body weight (`~40-70 kg`) of `ATP` molecules through mitochondrial `ATP Synthase` nanomotors spinning at `~100-150 rev/sec`.",
    ),
    (
        "Neuroscience, Immunology & Medicine",
        "Neuroscience & Medicine: Brain Neurons, Synapses, Immune System, Vaccines & Pharmacology",
        ["neuron", "neurons", "brain", "synapse", "dopamine", "serotonin", "neuroplasticity", "immune", "vaccine", "vaccines", "antibody", "tcell", "bcell", "virus", "bacteria", "antibiotic", "sleep"],
        ["immune system", "how vaccines work", "mrna vaccine", "action potential", "nervous system", "virus vs bacteria", "antibiotics vs antivirals"],
        "Human physiology coordinates fast electrochemical computation in the nervous system (`86 billion` neurons) and adaptive molecular defense in the immune system (`Innate` + `Adaptive` immunity).",
        "- **Neuroscience & Synaptic Plasticity**: Neurons maintain a `-70 mV` resting potential via the `Na+/K+ ATPase` pump. When excitatory inputs cross the `-55 mV` threshold, voltage-gated `Na+` channels snap open, firing an all-or-nothing **Action Potential** down the myelinated axon (`saltatory conduction`). At the synapse, `Ca2+` influx triggers vesicle release of neurotransmitters (`Glutamate` excitatory, `GABA` inhibitory, `Dopamine` reward-prediction error, `Serotonin` mood, `Acetylcholine` attention). **Hebbian Neuroplasticity (`LTP`)** strengthens co-active synapses (*'neurons that fire together wire together'*).\n- **Innate vs. Adaptive Immune System**: **Innate immunity** (skin barriers, macrophages, neutrophils, NK cells, complement, fever) responds immediately within hours. **Adaptive immunity** uses somatic V(D)J recombination to generate billions of unique receptors: `CD4+ Helper T-cells` orchestrate cytokines, `CD8+ Cytotoxic T-cells` destroy virus-infected cells presenting foreign peptides on `MHC-I`, and `B-cells` differentiate into plasma cells secreting neutralizing **Antibodies (`IgG`, `IgM`, `IgA`)** plus long-lived **Memory B/T cells**.\n- **Vaccines (`mRNA`, Subunit, Attenuated)**: Safely train Memory B and T cells on pathogen antigens (e.g. lipid-nanoparticle `mRNA` transiently translated in cytoplasm—never entering the nucleus or altering DNA) so real exposure triggers a rapid secondary immune response.",
        "Epidemiological herd immunity threshold: `p_c = 1 - 1 / R_0`, where `R_0` is the basic reproduction number.",
        "**Bacteria** are free-living prokaryotic cells with peptidoglycan cell walls and `70S` ribosomes—treatable with **Antibiotics** (e.g. penicillin blocking cell-wall synthesis). **Viruses** are obligate intracellular genetic packets (`DNA` or `RNA` in a protein capsid) that hijack host ribosomes—antibiotics do *nothing* against viruses; they require vaccines or targeted antivirals (protease/polymerase inhibitors).",
        "During deep slow-wave and REM sleep, the brain's **glymphatic system** increases interstitial fluid flow by `~60%` to clear metabolic byproducts (`beta-amyloid`) while consolidating hippocampal episodic memories into the neocortex.",
    ),
    (
        "Earth Science, Climate & Ecology",
        "Earth Science: Plate Tectonics, Climate Science, Carbon Cycle & Ecosystems",
        ["tectonics", "earthquake", "volcano", "climate", "greenhouse", "carbon", "atmosphere", "ocean", "ecosystem", "biodiversity", "weather", "elnino"],
        ["plate tectonics", "climate change", "greenhouse effect", "carbon cycle", "weather vs climate", "water cycle"],
        "Earth functions as a coupled geophysical and biogeochemical system driven by internal mantle convection (`Plate Tectonics`) and external solar radiative forcing (`Atmospheric & Ocean Circulation`).",
        "- **Plate Tectonics**: Mantle convection currents drive lithospheric plates across the asthenosphere at `2-10 cm/year`. **Divergent boundaries** form mid-ocean ridges; **Convergent subduction zones** generate deep trenches, volcanic arcs (Ring of Fire), and megathrust earthquakes; **Transform faults** (San Andreas) slide horizontally.\n- **Radiative Forcing & Greenhouse Effect**: Incoming shortwave solar radiation (`~340 W/m^2` average) warms Earth's surface, which re-emits longwave thermal infrared (`~10 um`). Triatomic and polyatomic greenhouse gases (`H2O`, `CO2`, `CH4`, `N2O`) have asymmetric vibrational dipole modes that absorb and re-radiate infrared heat.\n- **Weather vs. Climate**: *Weather* is the chaotic short-term atmospheric state (bounded by the ~10-14 day Lyapunov predictability horizon); *Climate* is the long-term (`30+ year`) statistical boundary distribution of energy, temperature, and precipitation.",
        "Planetary radiative equilibrium: `T_eq = [ S_0 (1 - Albedo) / (4 * sigma) ]^(1/4)` (`~255 K` or `-18 °C` without greenhouse warming; `+15 °C` with Earth's natural greenhouse effect; `Delta F = 5.35 * ln(C / C_0) W/m^2` for `CO2`).",
        "Decarbonizing global energy combines renewable generation (solar PV, wind, hydro), firm zero-carbon baseload (nuclear, geothermal), grid-scale storage (batteries, pumped hydro), and electrification of transport and heat.",
        "Ice cores from Antarctica (`EPICA Dome C`) preserve trapped atmospheric air bubbles spanning `800,000 years`, providing a direct physical record of historical `CO2`, `CH4`, and isotope (`delta-18O`) temperatures.",
    ),

    # ==================== WORLD HISTORY, ECONOMICS, PHILOSOPHY & PSYCHOLOGY ====================
    (
        "World History & Civilizations",
        "World History: Ancient Civilizations, Greece & Rome, Renaissance, Industrial Revolution & Modern Era",
        ["history", "roman", "rome", "greece", "greek", "egypt", "mesopotamia", "medieval", "renaissance", "enlightenment", "industrial", "revolution", "ww1", "ww2", "coldwar", "empire"],
        ["roman empire", "fall of rome", "industrial revolution", "ancient greece", "ancient egypt", "world war", "scientific revolution", "silk road"],
        "Human history traces a succession of technological, institutional, and intellectual transitions—from the Neolithic Agricultural Revolution (`~10,000 BCE`) to Classical Antiquity, the Scientific & Industrial Revolutions, and the modern digital information age.",
        "- **River Valley Civilizations (`~3500–1200 BCE`)**: Irrigation agriculture in Mesopotamia (Tigris/Euphrates, cuneiform, Hammurabi's Code), Ancient Egypt (Nile, hieroglyphs, monumental engineering), the Indus Valley (Harappa/Mohenjo-daro urban grids), and Yellow River China catalysed writing, metallurgy (Bronze/Iron), and state administration.\n- **Classical Greece & The Roman Empire**: Athens pioneered democratic governance, deductive geometry (Euclid, Archimedes), and philosophy (Socrates, Plato, Aristotle). Rome scaled republican/imperial law (`Corpus Juris Civilis`), continental road/aqueduct engineering, and Mediterranean trade. The **Western Roman Empire's fall (`476 CE`)** resulted from interacting structural pressures: currency debasement and inflation, military overextension along the Rhine-Danube frontier, civil wars over imperial succession, tax-base erosion, and Migration Period invasions—while the **Eastern Roman (Byzantine) Empire** endured another thousand years until `1453`.\n- **Renaissance, Printing Press & Scientific Revolution (`1450–1700`)**: Gutenberg's movable-type press (`~1450`) reduced book costs by `>90%`, accelerating the Renaissance, Reformation, empirical astronomy/physics (Copernicus, Galileo, Kepler, Newton), and Enlightenment constitutional theory.\n- **The Industrial Revolution (`1760–1914`)**: First wave (Britain: Watt's steam engine, coal, mechanized textiles, railways); Second wave (steel, electricity, internal combustion, chemical synthesis, telegraphy), multiplying global per-capita output after millennia of Malthusian stagnation.",
        "Institutional growth driver: societies that combine open scientific inquiry, secure property/contract law, low-cost information networks, and energy-dense prime movers achieve sustained compounding productivity.",
        "Single-cause historical explanations rarely hold; major historical turning points emerge from the interplay of **geography/ecology**, **energy/technology**, **demographics/epidemiology**, and **political/fiscal institutions**.",
        "Gutenberg's printing press (`~1450`) and the Internet (`1990s`) produced parallel structural effects: orders-of-magnitude drops in marginal publishing cost followed by rapid scientific acceleration and institutional realignment.",
    ),
    (
        "Economics, Finance & Business Strategy",
        "Economics & Finance: Supply/Demand, Inflation, Monetary Policy, Compound Interest & Strategy",
        ["economics", "economy", "inflation", "interest", "fed", "gdp", "supply", "demand", "elasticity", "tariff", "trade", "ricardo", "finance", "npv", "investing", "stock", "bond", "startup", "moat"],
        ["supply and demand", "monetary policy", "fiscal policy", "compound interest", "comparative advantage", "keynesian economics", "time value of money", "unit economics"],
        "Economics and finance analyze how scarce resources are allocated through price signals (`Microeconomics`), how aggregate output, inflation, and employment fluctuate (`Macroeconomics`), and how capital is valued across time and risk (`Finance`).",
        "- **Microeconomics**: Market prices equilibrate where quantity supplied equals quantity demanded (`Q_s(P*) = Q_d(P*)`). **Price Elasticity** (`E_d = (% change in Q) / (% change in P)`) determines tax incidence and pricing power. **Comparative Advantage (David Ricardo)** proves that two parties both gain from trade whenever their relative opportunity costs differ—even if one party is more productive at everything in absolute terms.\n- **Macroeconomics & Central Banking**: `GDP = C + I + G + (X - M)`. **Monetary Policy** (central banks adjusting policy interest rates and balance sheets) and **Fiscal Policy** (government taxation and spending) manage the business cycle. By the Taylor Rule and Fisher Equation (`r_real approx i_nominal - pi_inflation`), raising real interest rates cools excess aggregate demand and anchors inflation expectations.\n- **Finance & Investing**: Because money today can earn a return, future cash flows `CF_t` are discounted to **Net Present Value (`NPV = sum CF_t / (1 + r)^t`)**. **Compound Interest (`A = P (1 + r)^n`)** and low-cost diversified index funds exploit the equity risk premium while minimizing idiosyncratic company risk.",
        "Compound Growth: `A = P * (1 + r/m)^(m*t)` (`Rule of 72`: doubling time `T_double approx 72 / r_%`). SaaS Unit Economics: sustainable growth requires `LTV / CAC >= 3` and `CAC Payback <= 12-18 months`.",
        "**Keynesian economics** emphasizes counter-cyclical fiscal stimulus during liquidity traps when private demand collapses; **Monetarist / Neoclassical economics** (Friedman, Lucas) emphasizes long-run money-supply discipline, supply-side incentives, and rational expectations.",
        "Investing `$500/month` at an `8%` annualized real return compounds to `~$745,000` over `30 years` (`$180,000` principal contributed + `$565,000` compound growth).",
    ),
    (
        "Philosophy, Ethics & Epistemology",
        "Philosophy: Epistemology, Scientific Method, Ethics (Utilitarianism, Deontology, Virtue) & Stoicism",
        ["philosophy", "epistemology", "ethics", "morality", "utilitarianism", "deontology", "kant", "virtue", "aristotle", "stoicism", "existentialism", "camus", "nietzsche", "popper", "consciousness", "meaning"],
        ["meaning of life", "utilitarianism vs deontology", "categorical imperative", "virtue ethics", "falsifiability", "scientific method", "hard problem of consciousness", "stoic philosophy"],
        "Philosophy examines the nature of knowledge and scientific truth (`Epistemology`), right action and human flourishing (`Ethics`), the structure of reality (`Metaphysics`), and the relationship between mind and matter (`Philosophy of Mind`).",
        "- **Epistemology & Science**: Bridges **Rationalism** (deductive a priori proofs: Descartes, Leibniz, Lean 4) and **Empiricism** (inductive sensory observation: Locke, Hume). **Karl Popper's Falsifiability** criterion states that a scientific theory can never be inductively verified with 100% certainty, but must make risky, precise predictions that *could* be empirically refuted.\n- **Three Major Ethical Frameworks**:\n  1. **Consequentialism / Utilitarianism (Bentham, Mill, Singer)**: Judges actions by their outcomes—maximizing aggregate well-being and minimizing suffering across all sentient beings.\n  2. **Deontology (Immanuel Kant)**: Judges actions by intrinsic moral duties and universalizable rules (**Categorical Imperative**: *act only on maxims you could will to become universal law*, and *always treat humanity as an end in itself, never merely as a means*).\n  3. **Virtue Ethics (Aristotle, Confucius)**: Focuses on cultivating moral character and practical wisdom (*phronesis*) to achieve flourishing (*eudaimonia*).\n- **Stoicism & Existentialism**: **Stoicism** (Marcus Aurelius, Epictetus, Seneca) applies the *Dichotomy of Control*—directing full energy toward one's own judgments and actions while accepting external events with equanimity. **Existentialism** (Sartre, Camus, Viktor Frankl) holds that *existence precedes essence*: humans create authentic meaning through chosen commitments and responsibility.",
        "Occam's Razor (Solomonoff Induction / Minimum Description Length): among hypotheses that explain the data equally well, prefer the one with the shortest formal specification.",
        "Pure Utilitarianism handles resource allocation well but can struggle with individual rights constraints; strict Deontology protects inviolable rights but struggles when two duties conflict; combining **rule-consequentialist incentives** with **deontological guardrails** and **virtuous character** provides a balanced decision framework.",
        "Viktor Frankl's *Man's Search for Meaning* identifies three sources of human meaning: purposeful creative work, deep connection and love, and courage in the face of unavoidable adversity.",
    ),
    (
        "Psychology, Cognitive Science & Linguistics",
        "Psychology & Cognition: System 1 vs. System 2, Cognitive Biases, Memory & Linguistics",
        ["psychology", "cognitive", "bias", "biases", "kahneman", "heuristic", "confirmation", "anchoring", "survivorship", "dissonance", "memory", "spaced", "linguistics", "chomsky", "syntax"],
        ["cognitive bias", "system 1 and system 2", "thinking fast and slow", "confirmation bias", "spaced repetition", "how memory works", "chomsky hierarchy"],
        "Cognitive psychology and linguistics study how the mind perceives, encodes memory, makes judgments under uncertainty (`Dual-Process Theory`), and structures human language.",
        "- **Dual-Process Theory (Kahneman & Tversky)**: **System 1** is fast, automatic, associative, and emotional; **System 2** is slow, deliberative, serial, and logical. While System 1 heuristics are efficient in everyday life, they produce predictable **Cognitive Biases**:\n  - *Confirmation Bias*: Seeking evidence that confirms existing beliefs while discounting disconfirming data.\n  - *Anchoring & Availability*: Overweighting the first number heard or the most vivid/recent examples.\n  - *Survivorship Bias (Abraham Wald)*: Drawing conclusions only from visible 'winners' while ignoring unseen failures.\n  - *Sunk Cost Fallacy & Loss Aversion*: Losses hurt `~2x` more psychologically than equivalent gains (`Prospect Theory`).\n- **Memory & Learning Science**: Moving knowledge from working memory (`~4-7` chunks) into durable neocortical long-term memory is maximized by **Active Recall (Retrieval Practice)** and **Spaced Repetition** (reviewing at expanding intervals `1d -> 3d -> 7d -> 21d` along Ebbinghaus's forgetting curve), which beats passive re-reading by `2x-3x`.\n- **Linguistics**: Natural languages layer *Phonology* (sounds) -> *Morphology* (word roots/affixes) -> *Syntax* (hierarchical phrase trees; **Chomsky Hierarchy**: Regular `Type-3` < Context-Free `Type-2` < Context-Sensitive `Type-1` < Recursively Enumerable `Type-0`) -> *Semantics & Pragmatics*.",
        "Ebbinghaus Forgetting Curve: `R(t) = exp(-t / S)`, where memory stability `S` doubles with each successful active-recall retrieval.",
        "To debias high-stakes engineering or business decisions, replace unstructured group discussion with **independent written estimates**, **base-rate reference classes**, and a **Pre-Mortem** (*'imagine this project failed 6 months from now—write down why'*).",
        "During WWII, statistician Abraham Wald realized the military should armor the areas of returning bombers with *zero* bullet holes—because planes hit in those spots (engines and cockpit) never returned (`Survivorship Bias`).",
    ),
]


GLM_LOGIC_PUZZLES = [
    (
        "Probability & Decision Puzzle · The Monty Hall Problem",
        ["monty", "goat", "doors"],
        ["monty hall", "three doors", "switch doors"],
        """### The Monty Hall Problem — Exact Bayesian Solution

**Verdict:** You should **always switch doors**. Switching wins the car **`2/3` (`66.7%`)** of the time, whereas staying with your initial pick wins only **`1/3` (`33.3%`)** of the time.

#### Step-by-Step Proof
1. **Initial Prior**: When you first pick a door (say **Door 1**), the probability that the car is behind Door 1 is **`P(C = 1) = 1/3`**, and the probability that the car is behind **one of the other two doors (`Door 2 or Door 3`)** is **`2/3`**.
2. **Monty's Informed Action**: The host *knows* where the car is and *must* open a door that (a) you did not pick and (b) hides a goat. Suppose Monty opens **Door 3** to reveal a goat.
3. **Probability Mass Conservation**:
   - The `1/3` probability on your original **Door 1** never changes.
   - The entire **`2/3` probability mass** that belonged to `{Door 2, Door 3}` is now concentrated onto the single remaining closed door (**Door 2**), because Monty could never have opened Door 2 if the car were behind it!
4. **Intuitive 100-Door Generalization**: Imagine **100 doors**. You pick Door 1 (`1%` chance). Monty—knowing where the car is—opens **98 other doors** showing goats, leaving only **Door 1** and **Door 73** closed. Door 73 now holds **`99%`** of the probability mass!""",
    ),
    (
        "Combinatorial Algorithm Puzzle · Two Eggs and 100 Floors",
        ["eggs", "egg", "floors"],
        ["two eggs", "100 floors", "egg drop"],
        """### Two Eggs and 100 Floors — Minimax Optimal Strategy

**Verdict:** The minimum number of drops required in the worst case is **`14 drops`**.

#### Why Fixed-Step Intervals (Every 10 Floors) Are Suboptimal
- If you drop the first egg at floors `10, 20, ..., 100` and it breaks at floor `100`, you must test floors `91..99` one by one with the second egg — taking `10 + 9 = 19 drops` in the worst case.

#### The Optimal Triangular-Step Strategy (`14 Drops`)
To balance the worst-case total drops no matter which floor breaks the first egg, decrease the step size by **`1`** each time you move up (`x, x - 1, x - 2, ..., 1`) so that every drop of Egg 1 budgets one fewer linear scan step for Egg 2:
1. Solve the triangular sum inequality:
   `x + (x - 1) + (x - 2) + ... + 1 = x * (x + 1) / 2 >= 100`
2. For `x = 13`, `13 * 14 / 2 = 91 < 100`. For **`x = 14`**, **`14 * 15 / 2 = 105 >= 100`**.
3. **Drop Schedule for Egg 1**: Floors **`14, 27, 39, 50, 60, 69, 77, 84, 90, 95, 99, 100`** (`+14, +13, +12, +11, ...`).
   - If Egg 1 breaks on the very first drop at **Floor 14**, Egg 2 tests floors `1..13` linearly (`1 + 13 = 14` drops max).
   - If Egg 1 breaks on the 5th drop at **Floor 50**, Egg 2 tests floors `40..49` (`5 + 10 - 1 = 14` drops max).""",
    ),
    (
        "Constraint Satisfaction Puzzle · Wolf, Goat, and Cabbage River Crossing",
        ["wolf", "goat", "cabbage"],
        ["river crossing", "wolf goat", "farmer river"],
        """### Wolf, Goat, and Cabbage River Crossing — Minimum 7-Crossing Solution

**Constraints:** The boat holds only the **Farmer + at most 1 item**. Without the Farmer present, the **Wolf eats the Goat**, and the **Goat eats the Cabbage**.

#### Optimal 7-Step State Trajectory (`BFS` Shortest Path)
Notice that the **Goat** is the only item involved in *both* conflict edges (`Wolf-Goat` and `Goat-Cabbage`), so the Farmer must isolate the Goat first:

1. **Crossing 1 (`->`)**: Farmer takes the **Goat** across to the Far Bank (leaving Wolf & Cabbage safely together).
2. **Crossing 2 (`<-`)**: Farmer returns **alone** to the Near Bank.
3. **Crossing 3 (`->`)**: Farmer takes the **Wolf** (or Cabbage) across to the Far Bank.
4. **Crossing 4 (`<-`)**: Farmer brings the **Goat back** to the Near Bank (leaving the Wolf alone on the Far Bank!).
5. **Crossing 5 (`->`)**: Farmer takes the **Cabbage** across to the Far Bank (leaving the Goat on the Near Bank; Wolf & Cabbage stay safely together).
6. **Crossing 6 (`<-`)**: Farmer returns **alone** to the Near Bank.
7. **Crossing 7 (`->`)**: Farmer takes the **Goat** across to the Far Bank. **All four are safely across!**""",
    ),
    (
        "Probability Puzzle · The Birthday Paradox",
        ["birthday", "birthdays"],
        ["birthday paradox", "birthday problem", "23 people", "same birthday"],
        """### The Birthday Paradox — Why Only 23 People Give a `>50%` Match

**Verdict:** In a room of just **`23 people`**, there is a **`50.73%`** probability that at least two people share the same birthday. With **`70 people`**, the probability reaches **`99.9%`**!

#### Why Our Intuition Misses It
Human intuition compares *one* person against `22` others (`22 / 365 approx 6%`). In reality, **any pair** among the 23 people can match, and there are **`C(23, 2) = (23 * 22) / 2 = 253` distinct pairs**!

#### Exact Complement Calculation
Compute the probability `P(no match)` that all `N = 23` birthdays are mutually distinct:
- `P(no match) = (365/365) * (364/365) * (363/365) * ... * (343/365) = 0.4927`
- `P(at least one match) = 1 - 0.4927 = 0.5073` (**`50.73%`**)

*(In cryptography, this same square-root `sqrt(N)` collision scaling is the **Birthday Bound**: a `256-bit` hash like `SHA-256` or `BLAKE3` has `128 bits` (`2^128`) of collision resistance.)*""",
    ),
    (
        "Deductive Logic Puzzle · Two Doors, One Truth-Teller and One Liar",
        ["guards", "knights", "knaves", "liar"],
        ["two doors", "two guards", "truth teller", "one tells the truth"],
        """### Two Doors (Truth-Teller & Liar) — Single-Question Solution

**Setup:** Two doors (**Door A** and **Door B**); one leads to freedom, one to a trap. Guard 1 **always tells the truth (`T`)**; Guard 2 **always lies (`L`)**, and you don't know which guard is which. You may ask **one yes/no question** to one guard.

#### The Winning Question
Ask either guard:
> ***\"If I were to ask the OTHER guard whether Door A leads to freedom, would they say YES?\"***

#### Logical Proof (`Double Negation Composition T(L(x)) == L(T(x)) == NOT x`)
Because the question routes through **both** the Truth-Teller (`identity`) and the Liar (`logical NOT`) in series, the composite answer is **guaranteed to be the exact opposite (`NOT`) of the truth**, regardless of which guard you ask:
- **Case 1 (Door A is truly Safe):**
  - If you ask the **Truth-Teller**, they truthfully report that the Liar would lie and say **`"No"`**.
  - If you ask the **Liar**, they lie about the Truth-Teller's `"Yes"` and also say **`"No"`**.
- **Case 2 (Door A is a Trap):**
  - Both guards will answer **`"Yes"`**.
- **Action Rule:** If the guard answers **`"No"`**, take **Door A**; if **`"Yes"`**, take **Door B**!""",
    ),
    (
        "Information Theory Puzzle · Counterfeit Coin / Balance Scale Puzzle",
        ["coins", "counterfeit", "scale", "weighings"],
        ["12 coins", "8 coins", "9 coins", "balance scale", "counterfeit coin"],
        """### Counterfeit Coin Balance Scale Puzzle — Ternary (`Base-3`) Information Proof

**Core Information Principle:** A two-pan balance scale has **3 possible outcomes** per weighing: *Left Heavy (`-1`)*, *Balanced (`0`)*, or *Right Heavy (`+1`)*—just like the **balanced ternary `{-1, 0, +1}` binder gates (`R4.Router`)** in UOR-R4!

1. **8 or 9 Coins (1 Counterfeit Known to be Heavier) in `2 Weighings` (`3^2 = 9`)**:
   - **Weighing 1**: Divide into three groups of 3 (`A = {1,2,3}`, `B = {4,5,6}`, `C = {7,8,9}`). Weigh `A` vs `B`.
     - If `A == B`, the heavy coin is in `C`; if `A > B`, it is in `A`; if `B > A`, it is in `B`.
   - **Weighing 2**: Take the winning group of 3 and weigh `Coin 1` vs `Coin 2`. The heavier pan (or `Coin 3` if balanced) identifies the exact coin in 2 steps!
2. **12 Coins (1 Counterfeit, Unknown Whether Heavier or Lighter) in `3 Weighings` (`(3^3 - 3)/2 = 12`)**:
   - **Weighing 1**: Weigh `{1, 2, 3, 4}` vs `{5, 6, 7, 8}`.
     - If balanced, `{1..8}` are all genuine and the odd coin is in `{9, 10, 11, 12}` (solved in 2 weighings against known genuine coins).
     - If unbalanced (say Left goes down), `{9..12}` are genuine, `{1..4}` are potentially heavy, and `{5..8}` are potentially light.
   - **Weighing 2**: Rotate 3 coins: weigh `{1, 2, 5}` (2 heavy-candidates + 1 light-candidate) vs `{3, 6, 9}` (1 heavy-candidate + 1 light-candidate + 1 genuine) to narrow down to 3 possibilities resolved in **Weighing 3**.""",
    ),
    (
        "Identity & Metaphysics Paradox · The Ship of Theseus",
        ["theseus", "planks"],
        ["ship of theseus", "theseus paradox"],
        """### The Ship of Theseus — Four Philosophical & Formal Resolutions

**The Paradox:** If every wooden plank of Theseus's ship is gradually replaced over decades as it decays, is the fully restored vessel still the *same* Ship of Theseus? And if someone reassembles the discarded original planks into a second ship, which one is the true Ship of Theseus?

#### Four Complementary Resolutions
1. **Aristotle's Four Causes**: The *material cause* (physical planks) changed, but the *formal cause* (structural blueprint/design) and *final cause* (telos/purpose as Theseus's sailing vessel) remained invariant.
2. **Spacetime Worms / 4D Perdurantism (Ted Sider, David Lewis)**: An object is a 4-dimensional spacetime trajectory. The continuously maintained ship has an unbroken causal-functional trajectory, whereas the reassembled museum ship is a historical reconstruction branching from old material parts.
3. **Biological Analogy**: Almost every cell and molecule in the human body (`ATP`, lipids, epithelial cells, proteins) is replaced over years, yet personal identity persists through continuous homeostatic and informational continuity.
4. **Computer Science / Content-Addressable Resolution (`Git` & `R4.Object`)**: Distinguishes **Mutable Identity (Reference/Branch Pointer `HEAD`)** from **Immutable Value Identity (Cryptographic Merkle Hash `SHA-256`/`BLAKE3`)**—the two ships simply have different cryptographic state digests at time `t`, eliminating any ambiguity!""",
    ),
]


GLM_CONCEPTS_PART3 = [
    # ==================== GRANULAR LANGUAGE, SYSTEM & SCIENCE NODES FOR PAIRWISE SYNTHESIS ====================
    (
        "Programming Languages · Systems",
        "Rust Programming Language (Ownership, Borrow Checker & Zero-Cost Abstractions)",
        ["rust", "cargo", "rustc", "borrowchecker", "lifetimes", "tokio", "trait", "traits"],
        ["rust language", "rust ownership", "borrow checker"],
        "Rust is a statically typed systems programming language focused on performance, reliability, and compile-time memory safety without a garbage collector.",
        "The Rust compiler (`rustc`) enforces Ownership, Borrowing (`&T` vs `&mut T`), and Lifetimes (`'a`) at compile time, freeing heap values deterministically via `Drop` (`RAII`) when their owner exits scope. Traits provide zero-cost generic polymorphism via monomorphization, and `Result<T, E>` / `Option<T>` enforce exhaustive error handling.",
        "Compile-time aliasing invariant: `forall ptr, Readers(ptr) > 0 => Writers(ptr) == 0`. Runtime GC pause overhead: `0 ms`.",
        "Strengths: C/C++-class execution speed, zero garbage-collection tail latency, fearless concurrency, and first-class WebAssembly compilation. Trade-offs: stricter compile-time borrow-checker discipline and longer clean-build times than Go.",
        "Used in the Linux kernel, Cloudflare edge proxies, AWS Firecracker microVMs, and this `lexlean-r4-wasm` engine.",
    ),
    (
        "Programming Languages · Cloud & Services",
        "Go (Golang) Programming Language (Goroutines, Channels & Cloud Services)",
        ["go", "golang", "goroutine", "goroutines", "gopher"],
        ["go language", "go programming", "goroutines and channels"],
        "Go (`Golang`) is a statically typed, compiled programming language designed at Google for fast compilation, readable concurrency, and scalable networked cloud infrastructure.",
        "Go's runtime multiplexes hundreds of thousands of lightweight user-space **goroutines** (`~2 KB` growable stacks) onto OS threads using a work-stealing `M:N` scheduler (`G-M-P` model), communicating via typed `chan T` CSP channels and a low-pause concurrent tri-color mark-and-sweep garbage collector.",
        "Goroutine spawn overhead: `~2 KB` initial stack vs `1-8 MB` for an OS thread (`~1,000x` denser concurrency).",
        "Strengths: sub-second compilation to a single static binary, simple syntax, effortless concurrent I/O, and rich standard HTTP/networking tooling. Trade-offs: garbage collector incurs minor tail latency (`~0.5-2 ms`) and higher memory headroom compared to Rust.",
        "Powers Docker, Kubernetes, etcd, Prometheus, Terraform, and CockroachDB.",
    ),
    (
        "Programming Languages · AI & Scripting",
        "Python Programming Language (Dynamic Typing, Asyncio, PyTorch & Data Stack)",
        ["python", "cpython", "gil", "asyncio", "pytorch", "numpy", "pandas", "fastapi", "pip", "uv"],
        ["python language", "python programming", "global interpreter lock"],
        "Python is an expressive, high-level language with dynamic execution and optional static type annotations that serves as the universal glue language for AI/ML, scientific computing, backend APIs, and automation.",
        "CPython compiles source into bytecode executed by a stack virtual machine (`ceval.c`) using reference counting plus a cyclic garbage detector. Heavy numerical and tensor workloads (`NumPy`, `PyTorch`, `Polars`, `CUDA`) bypass interpreter overhead by dispatching vectorized array operations directly to native C/C++/Rust/CUDA kernels.",
        "Vectorized execution: moving a loop over `10^6` numbers from pure Python bytecode into a contiguous `NumPy` / `CUDA` array reduces runtime by `50x-500x`.",
        "Strengths: unmatched developer velocity, readability, and the richest AI/data science ecosystem. Trade-offs: pure-Python CPU loops are `20x-100x` slower than native Rust/C++ and historically constrained by the Global Interpreter Lock (`GIL`, now optional in free-threaded Python 3.13+).",
        "`FastAPI` + `Pydantic` (Rust core) + `PyTorch` is the standard stack for training and serving modern machine learning pipelines.",
    ),
    (
        "Programming Languages · Web & Full-Stack",
        "TypeScript & JavaScript (V8 JIT, Event Loop, Promises & Structural Typing)",
        ["typescript", "javascript", "v8", "nodejs", "eventloop", "promise", "promises", "react", "dom", "closure", "closures"],
        ["event loop", "typescript vs javascript", "javascript closures", "async await"],
        "JavaScript is the native runtime language of the Web, and TypeScript extends it with a rich compile-time structural type system, generics, and discriminated unions.",
        "Inside the `V8` engine, JavaScript source is parsed into bytecode (`Ignition`) and hot functions are JIT-compiled into optimized machine code (`Maglev` / `TurboFan`) using inline caches and hidden classes. Concurrency runs on a single-threaded non-blocking **Event Loop**: synchronous call-stack frames run to completion, followed by all queued **Microtasks** (`Promise.then`, `await`), and then the next **Macrotask** (`setTimeout`, I/O, UI render).",
        "Type erasure: TypeScript types exist purely at compile time (`0 bytes` runtime overhead) while catching null-dereference and schema mismatches statically.",
        "Strengths: universal browser execution, fast non-blocking I/O (`Node.js`, `Deno`, `Bun`), and ergonomic structural types. Trade-offs: single-threaded CPU execution requires `Web Workers` or `WebAssembly` (`lexlean-r4-wasm`) for heavy parallel compute.",
        "Combining a zero-dependency JavaScript/DOM frontend (`public/app.js`) with a WebAssembly compute engine delivers native speed inside any browser tab.",
    ),
    (
        "Networking & API Design",
        "REST vs. GraphQL vs. gRPC & WebSockets",
        ["rest", "restful", "graphql", "grpc", "protobuf", "websocket", "websockets", "webhook", "api"],
        ["rest vs graphql", "rest vs grpc", "graphql vs grpc", "api design"],
        "Modern distributed applications communicate across service boundaries using four primary API paradigms: **REST**, **GraphQL**, **gRPC**, and **WebSockets**.",
        "- **REST (`HTTP + JSON`)**: Resource-oriented URLs (`/users/42`) using standard HTTP verbs (`GET`, `POST`, `PUT`, `DELETE`), status codes, and HTTP caching (`ETag`, `Cache-Control`).\n- **GraphQL**: Client-driven query language over a typed schema graph where the frontend requests the exact nested fields it needs in a single round-trip (solving REST over-fetching and N+1 under-fetching, paired with `DataLoader` batching).\n- **gRPC (`HTTP/2 + Protocol Buffers`)**: Contract-first binary RPC with code-generated strongly typed stubs, bi-directional streaming, and `5x-10x` smaller/faster wire serialization than JSON.\n- **WebSockets / SSE**: Persistent full-duplex (`WebSocket`) or server-push (`Server-Sent Events`) channels for real-time low-latency updates.",
        "Serialization efficiency: binary `Protobuf` (`varint` + tag-length-value) is `~3x-8x` more compact and faster to parse than text `JSON`.",
        "Use **REST** for public developer APIs and CDN-cacheable resources; use **GraphQL** for complex UI aggregation layers; use **gRPC** for high-throughput internal service-to-service communication; use **WebSockets/SSE** for live streaming.",
        "In `UOR-R4`, canonical numeric and varint wire formats are formally verified under `uor-naf/1-draft.6` (`R4-27`) and `R4.Json` (`R4-21..R4-22`).",
    ),
    (
        "Web Development & Styling",
        "HTML5, CSS Flexbox & Grid Layout, Accessibility (WCAG) & Responsive Design",
        ["html", "css", "flexbox", "grid", "center", "div", "responsive", "accessibility", "wcag", "aria", "dom"],
        ["center a div", "flexbox vs grid", "css grid", "css flexbox", "web accessibility"],
        "Modern web interfaces combine semantic HTML5 landmarks, 1D (`Flexbox`) and 2D (`CSS Grid`) layout engines, CSS custom properties (variables), and WCAG 2.1 AA accessibility semantics.",
        "- **Centering a `<div>` (Grid vs. Flexbox)**:\n  - *CSS Grid (2 lines)*: `.parent { display: grid; place-items: center; }`\n  - *CSS Flexbox*: `.parent { display: flex; justify-content: center; align-items: center; }`\n- **Flexbox vs. Grid**: **Flexbox** (`display: flex`) lays out items along a single primary axis (row or column) with dynamic wrapping; **CSS Grid** (`display: grid`) controls rows and columns simultaneously in 2D (`grid-template-columns: repeat(auto-fit, minmax(280px, 1fr))`).\n- **Accessibility (`a11y`)**: Uses semantic elements (`<main>`, `<nav>`, `<aside>`, `<button>`), `aria-live=\"polite\"` regions for streaming chat updates, visible `:focus-visible` outlines, and `>= 4.5:1` contrast ratios.",
        "Browser rendering pipeline: `DOM + CSSOM -> Render Tree -> Layout (Reflow) -> Paint -> GPU Compositing (`transform` / `opacity` run at 60-120fps on the compositor thread without triggering Layout)`.",
        "Use **Flexbox** for navigation bars, button toolbars, and chat message rows; use **CSS Grid** for page-level shells, bento dashboards, and card galleries.",
        "The UOR-R4 web interface uses a responsive Flexbox/Grid layout with zero external CSS frameworks and strict Content-Security-Policy (`default-src 'self'`).",
    ),
    (
        "Literature, Storytelling, Music & Visual Arts",
        "Narrative Structure, Creative Writing, Rhetoric, Music Theory & Visual Harmony",
        ["story", "narrative", "plot", "character", "hero", "rhetoric", "metaphor", "poetry", "music", "harmony", "chord", "scale", "color", "design"],
        ["hero's journey", "three act structure", "music theory", "creative writing", "color theory", "golden ratio"],
        "Across literature, rhetoric, music, and visual design, compelling human expression arises from structured tension, symmetry, harmonic ratios, and resolution.",
        "- **Narrative Architecture**: Stories progress through **Setup & Inciting Incident** (disrupting equilibrium) -> **Rising Action & Progressive Complications** (stakes escalate as easy solutions fail) -> **Climax / Crisis** (the protagonist faces the core thematic choice) -> **Resolution & Transformation** (a new equilibrium with earned insight).\n- **Classical Rhetoric (Aristotle)**: Persuasive writing balances **Logos** (rigorous evidence and logical structure), **Ethos** (credibility and intellectual honesty), and **Pathos** (concrete sensory imagery and human stakes).\n- **Music Theory & Acoustics**: Musical intervals correspond to small-integer frequency ratios discovered by Pythagoras: **Octave (`2:1`)**, **Perfect Fifth (`3:2`)**, **Perfect Fourth (`4:3`)**, and **Major Third (`5:4`)**. Twelve-tone equal temperament spaces semitones by `2^(1/12)` so keys modulate seamlessly.",
        "Harmonic overtone series: `f_n = n * f_0` (`n = 1, 2, 3, ...`). Cadential tension-and-release (`Dominant V7 -> Tonic I`) mirrors narrative crisis-and-resolution.",
        "Strong writing replaces abstract adjectives with **specific nouns, active verbs, concrete sensory details, and varied sentence cadence**.",
        "Even in mathematical geometry, the **Icosian Golden Ratio `phi = (1 + sqrt(5)) / 2 approx 1.618`** (`R4-24`) governs both icosahedral symmetry and classical proportion.",
    ),
    (
        "Everyday Science · Culinary, Health, Fitness & Productivity",
        "Culinary Science (Sourdough, Coffee, Maillard), Exercise Physiology, Sleep & Decision Strategy",
        ["cooking", "sourdough", "bread", "coffee", "espresso", "fermentation", "fitness", "exercise", "muscle", "vo2max", "nutrition", "protein", "negotiation", "interview", "productivity"],
        ["sourdough bread", "how to make", "system design interview", "salary negotiation", "muscle hypertrophy", "sleep quality"],
        "Applying first-principles science transforms everyday pursuits—from artisan cooking and espresso extraction to strength/endurance training, sleep optimization, and high-stakes negotiation.",
        "- **Culinary Science (Sourdough & Espresso)**: Sourdough relies on a symbiotic culture of wild yeast (`Saccharomyces` producing `CO2` for rise) and lactic acid bacteria (`Lactobacillus` producing lactic/acetic acids for flavor and gluten conditioning), baked with steam at `230-250 °C` for oven spring and **Maillard browning**. Espresso extraction (`9 bar`, `92-94 °C`, `25-30 sec` for a `1:2` dose-to-yield ratio) balances early acidic fruit notes, mid-shot sweetness, and late bitter plant fibers.\n- **Exercise Physiology & Nutrition**: **Strength & Hypertrophy** are driven by mechanical tension, progressive overload (`6-15` reps near failure), and `~1.6-2.0 g/kg` daily protein intake (`leucine` mTOR trigger). **Cardiorespiratory Fitness (`VO2 Max`)** combines `80%` Zone 2 aerobic base (mitochondrial biogenesis) with `20%` high-intensity `VO2 Max` intervals (`4x4 min`).\n- **Negotiation & Strategy (`Harvard Principled Negotiation`)**: Know your **BATNA** (Best Alternative to a Negotiated Agreement), separate the people from the problem, focus on underlying *interests* rather than rigid *positions*, and anchor with objective criteria.",
        "Fick Equation for `VO2 Max`: `VO2 = Cardiac_Output (HR * Stroke_Volume) * (C_aO2 - C_vO2)` (oxygen delivery times muscular extraction).",
        "Consistency over months and years compounds far more strongly than unsustainable short-term intensity—in training, learning (`spaced repetition`), and engineering alike.",
        "A `5-10 minute` morning sunlight exposure anchors the suprachiasmatic nucleus (`SCN`) circadian clock, timing melatonin release `~14-16 hours` later for deep restorative sleep.",
    ),
]


GLM_CONCEPTS_PART4 = [
    (
        "Computer Science · Core Concepts",
        "Recursion, Base Cases & Tail-Call Optimization",
        ["recursion", "recursive", "basecase", "callstack", "tailcall", "divideandconquer"],
        ["what is recursion", "base case", "tail recursion", "divide and conquer"],
        "Recursion is a computational paradigm where a function solves a problem by decomposing it into smaller self-similar instances until reaching a terminating base case.",
        "Every well-founded recursive function requires two components: (1) a **Base Case** that returns a known value directly without recursing (e.g. `n <= 1`), and (2) a **Recursive Step** that shrinks a strictly decreasing measure toward the base case (e.g. `n - 1` or `n / 2`). When the recursive call is the very last operation in the function (**Tail Recursion**), compilers and proof assistants (`Lean 4`, `Rust`/`LLVM`) reuse the current stack frame in `O(1)` space via Tail-Call Optimization (`TCO`).",
        "Master Theorem for divide-and-conquer recurrences `T(N) = a * T(N / b) + O(N^d)`: if `d == log_b(a)`, then `T(N) = O(N^d log N)` (e.g. MergeSort has `a=2, b=2, d=1 => O(N log N)`).",
        "Unbounded recursion without a base case or with `O(N)` call-stack depth risks `StackOverflowError` on large inputs; converting tail recursion to an iterative loop or using an explicit heap stack guarantees `O(1)` call-stack safety.",
        "In `Lean 4` (`LexLean`), every recursive function must prove structural termination on a strictly decreasing argument, ensuring total mathematical soundness with zero infinite loops.",
    ),
    (
        "Computer Science · Complexity",
        "Big-O Asymptotic Time & Space Complexity",
        ["bigo", "complexity", "asymptotic", "logarithmic", "linear", "quadratic", "exponential", "amortized"],
        ["big o", "big-o", "time complexity", "space complexity", "amortized analysis"],
        "Big-O notation (`O(f(N))`) characterizes the upper-bound growth rate of an algorithm's execution time or memory footprint as input size `N` scales toward infinity.",
        "The major complexity classes in increasing order of cost are:\n- **`O(1)` Constant**: Array index, hash table lookup, `UOR-R4` `/7` table read.\n- **`O(log N)` Logarithmic**: Binary search on a sorted array, balanced `B-Tree` lookup (halving the search space each step).\n- **`O(N)` Linear**: Single-pass array scan, two-pointer palindrome check, `SWAR` Hamming scan.\n- **`O(N log N)` Linearithmic**: MergeSort, HeapSort, Fast Fourier Transform (`FFT`).\n- **`O(N^2)` Quadratic**: Nested all-pairs loops, Bubble/Insertion sort, dense Transformer self-attention.\n- **`O(2^N)` / `O(N!)` Exponential / Factorial**: Brute-force subset enumeration or Traveling Salesperson.",
        "Formal definition: `T(N) in O(f(N))` iff there exist constants `c > 0, N_0 > 0` such that `T(N) <= c * f(N)` for all `N >= N_0`.",
        "Asymptotic Big-O ignores constant factors and cache hardware behavior: for small `N <= 64`, a contiguous `O(N)` linear scan in L1 cache is often faster than an `O(log N)` pointer-chasing tree due to zero cache misses.",
        "Doubling `N` from `1,000` to `2,000` adds just `1` extra step to `O(log2 N)` binary search (`10 -> 11` comparisons), `2x` time to `O(N)`, and `4x` time to `O(N^2)`.",
    ),
    (
        "Software Engineering · Version Control",
        "Git Internals, Merkle DAGs & Branching Workflows (Merge vs. Rebase)",
        ["git", "commit", "branch", "merge", "rebase", "cherrypick", "bisect", "reflog", "stash"],
        ["git merge vs rebase", "how git works", "git rebase", "git bisect"],
        "Git is a distributed, content-addressable version control system that stores repository history as a cryptographic Merkle Directed Acyclic Graph (`DAG`) of immutable snapshots.",
        "Every file (`blob`), directory listing (`tree`), and history node (`commit`) is addressed by the cryptographic hash of its contents (`SHA-1` / `SHA-256`). Branches (`main`, `feature`) are simply lightweight `41-byte` mutable pointers to commit hashes. **`git merge`** joins two diverging branches with a two-parent merge commit preserving exact chronological topology; **`git rebase`** replays your feature branch's diffs atop the target branch tip to produce a clean, linear commit history.",
        "Golden Rule of Rebasing: **Never rebase commits that have already been pushed to a shared public branch**, because rebasing rewrites commit hashes and creates divergent duplicate histories for teammates.",
        "Use `git rebase -i` locally to clean up work-in-progress commits before opening a pull request, and use `git merge --no-ff` (or squash-merge) on `main` for auditable release history.",
        "When a regression appears, `git bisect` performs an automated `O(log2 N)` binary search across `1,000` commits to pinpoint the exact breaking commit in just `10` test runs!",
    ),
    (
        "Computer Science · System Design",
        "System Design Architecture (Rate Limiters, URL Shorteners, Chat & Caching)",
        ["systemdesign", "ratelimiter", "tokenbucket", "urlshortener", "loadbalancer", "cache", "redis", "cdn", "kafka", "backpressure"],
        ["system design", "rate limiter", "token bucket", "url shortener", "load balancer", "caching strategy"],
        "Large-scale system design combines stateless horizontal compute tiers, multi-tier caching (`CDN` + `Redis`), partitioned storage (`B-Tree`/`LSM`), and asynchronous event queues (`Kafka`) to achieve high throughput (`QPS`), low tail latency (`p99`), and `99.99%+` availability.",
        "- **Rate Limiting**: Implemented via **Token Bucket** (allows controlled bursts up to bucket capacity `B` refilling at rate `r` tokens/sec) or **Sliding Window Counter** in Redis using atomic Lua scripts.\n- **Caching Strategies**: *Cache-Aside* (app reads cache, on miss reads DB and populates cache), *Write-Through*, or *Write-Behind*. Prevent **Cache Stampedes (Thundering Herd)** on hot-key expiration using single-flight request coalescing (`mutex` lease) or probabilistic early recomputation (`jittered TTL`).\n- **High-Scale Read/Write Separation**: Route writes to the primary leader shard (or partitioned `Kafka` log), replicate asynchronously to read replicas, and serve static/idempotent responses from edge `CDNs`.",
        "Little's Law: `L = lambda * W` (average concurrent in-flight requests `L` equals arrival rate `lambda` times average latency `W`). Availability in series: `A_total = A_1 * A_2`; in active-active parallel redundancy: `A_total = 1 - (1 - A_1)*(1 - A_2)`.",
        "Every architectural choice trades off **Latency (`p99`)**, **Consistency (Freshness)**, **Throughput (`QPS`)**, and **Operational Complexity / Cost**.",
        "A `62-character` Base62 (`[0-9a-zA-Z]`) short-URL slug of length `7` provides `62^7 = 3.52 trillion` unique URLs without collisions when backed by a distributed Snowflake ID generator.",
    ),
    (
        "Physics · Nuclear Energy",
        "Nuclear Fission vs. Nuclear Fusion",
        ["fission", "fusion", "uranium", "plutonium", "deuterium", "tritium", "tokamak", "iter", "reactor", "nuclear"],
        ["fission vs fusion", "nuclear fission", "nuclear fusion"],
        "Nuclear Fission and Nuclear Fusion both release millions of times more energy per kilogram than chemical combustion by converting nuclear mass defect (`Delta m`) into kinetic energy (`E = Delta m * c^2`) toward the maximally stable **Iron-56 (`56Fe`)** binding-energy peak.",
        "- **Nuclear Fission**: Splits heavy, neutron-rich actinide nuclei (`Uranium-235` or `Plutonium-239`) via thermal or fast neutron capture into two lighter fission fragments plus `2-3` neutrons (`~200 MeV` per fission), sustaining a controlled chain reaction (`k_eff = 1.0`) moderated by water or graphite.\n- **Nuclear Fusion**: Fuses light hydrogen isotopes (**Deuterium `2H` + Tritium `3H` -> `4He` (`3.5 MeV`) + `n` (`14.1 MeV`)**) by overcoming electrostatic Coulomb repulsion at `>100 million K` inside magnetic confinement (`Tokamak` / `Stellarator`) or inertial laser confinement (`NIF`).",
        "Lawson Triple Product for Fusion Ignition: `n_e * T * tau_E >= 3 * 10^21 m^-3 keV s` (plasma density times temperature times energy confinement time).",
        "**Fission** is a mature, dispatchable zero-carbon baseload technology operating worldwide today, but requires long-term management of radioactive fission-product waste (`~300 year` decay for fission products). **Fusion** uses abundant fuel (Deuterium from seawater, Tritium bred from Lithium), cannot suffer a runaway meltdown, and produces no long-lived high-level actinide waste, but faces extreme plasma engineering and first-wall neutron materials challenges.",
        "One kilogram of Deuterium-Tritium fusion fuel releases `~3.4 * 10^14 Joules`—equivalent to burning **`~10,000 tons` of coal** with zero `CO2` emissions.",
    ),
    (
        "Biology · Molecular Genetics",
        "DNA vs. RNA (Structure, Function & The Genetic Code)",
        ["dna", "rna", "mrna", "trna", "rrna", "deoxyribose", "ribose", "uracil", "thymine", "nucleotide", "helix"],
        ["dna vs rna", "difference between dna and rna", "double helix"],
        "`DNA` (Deoxyribonucleic Acid) and `RNA` (Ribonucleic Acid) are the two complementary nucleic acid polymers that store, transcribe, regulate, and translate genetic information across all cellular life.",
        "- **Sugar Backbone & Stability**: `DNA` uses **2'-deoxyribose** (lacking the reactive `2'-OH` hydroxyl group), making it chemically ultra-stable for long-term archival genome storage across centuries. `RNA` uses **ribose** (with a reactive `2'-OH` group), making it suitable for transient messenger (`mRNA`), catalytic (`ribozyme` / `rRNA`), and regulatory (`miRNA`/`siRNA`) roles.\n- **Nucleobases & Strand Geometry**: `DNA` pairs Adenine with **Thymine (`A-T`, 2 H-bonds)** and Guanine with Cytosine (`G-C`, 3 H-bonds) in an antiparallel `B-form` **double helix**. `RNA` replaces Thymine with **Uracil (`A-U`)** and is predominantly single-stranded, folding into intricate 3D hairpins and catalytic pockets.",
        "Chargaff's Rules in dsDNA: `%A = %T` and `%G = %C`; higher `G-C` content (`3` hydrogen bonds vs `2`) increases the thermal melting temperature `T_m` of the duplex.",
        "`DNA` is the immutable archival blueprint stored safely in the nucleus (`Read-Only Storage`); `RNA` is the versatile working copy, splicing adapter, and catalytic ribosome assembler (`RAM & Execution Engine`).",
        "The **RNA World Hypothesis** proposes that early life used RNA for both genetic storage and enzymatic catalysis (`ribozymes`) before evolving stable `DNA` and protein enzymes.",
    ),
]


GLM_CONCEPTS_PART5 = [
    (
        "Biology · Cell Division",
        "Mitosis vs. Meiosis (Somatic Division vs. Gametogenesis)",
        ["mitosis", "meiosis", "chromosomes", "diploid", "haploid", "gametes", "prophase", "metaphase", "anaphase", "telophase", "crossingover"],
        ["mitosis vs meiosis", "cell division", "homologous recombination"],
        "Mitosis and Meiosis are the two eukaryotic cell-division programs: **Mitosis** clones somatic cells for growth and tissue repair, while **Meiosis** halves chromosome number and shuffles alleles to produce genetically diverse gametes (`sperm` and `egg`).",
        "- **Mitosis (`1 Division -> 2 Identical Diploid 2n Cells`)**: After `S-phase` DNA replication, sister chromatids align at the metaphase plate and separate once (`Prophase -> Metaphase -> Anaphase -> Telophase`), preserving exact `2n = 46` human chromosome identity.\n- **Meiosis (`2 Divisions -> 4 Unique Haploid 1n Cells`)**: In **Meiosis I**, homologous maternal and paternal chromosomes pair up (*synapsis*) and exchange DNA segments via **Crossing-Over (Homologous Recombination)** before separating independently (*Independent Assortment*). **Meiosis II** then separates sister chromatids into four `1n = 23` haploid gametes.",
        "Combinatorial genetic diversity: in humans (`n = 23`), independent assortment alone yields `2^23 = 8,388,608` chromosome combinations per gamete (`2^46 approx 70 trillion` zygote combinations before even counting crossing-over!).",
        "Mitosis prioritizes **genetic fidelity** across tissue renewal; Meiosis prioritizes **combinatorial evolutionary diversity** and ploidy reduction so fertilization (`1n + 1n = 2n`) restores the diploid genome.",
        "Nondisjunction errors during Meiosis I or II cause aneuploidies such as Trisomy 21, whereas unregulated mitotic checkpoints (`p53`, `Rb` mutations) drive oncogenic tumor proliferation.",
    ),
    (
        "Biology & Medicine · Microbiology",
        "Bacteria vs. Viruses & Antibiotics vs. Antivirals",
        ["bacteria", "bacterium", "virus", "viruses", "pathogen", "prokaryote", "capsid", "antibiotic", "antibiotics", "antiviral", "penicillin", "bacteriophage"],
        ["bacteria vs viruses", "virus vs bacteria", "how antibiotics work", "antibiotic resistance"],
        "Bacteria and Viruses are fundamentally different biological entities with completely distinct replication mechanisms and medical treatments.",
        "- **Bacteria (`~1,000–5,000 nm`, Living Prokaryotic Cells)**: Possess their own circular `DNA` genome, `70S` ribosomes, metabolic enzymes, and a rigid **peptidoglycan cell wall**, reproducing independently by binary fission. Treatable with **Antibiotics** (e.g. *beta-lactams* like penicillin inhibit cell-wall crosslinking; *macrolides/tetracyclines* target bacterial `70S` ribosomes without harming human `80S` ribosomes).\n- **Viruses (`~20–300 nm`, Acellular Genetic Packets)**: Consist only of a `DNA` or `RNA` genome encased in a protein **capsid** (and sometimes a lipid envelope). Viruses have zero ribosomes and zero metabolism of their own; they must bind a host surface receptor (`ACE2`, `CD4`, sialic acid) and hijack the host cell's machinery to replicate.",
        "Selective Toxicity: antibiotics target structures unique to bacteria (`peptidoglycan`, `70S` ribosome, bacterial `DNA gyrase`). Because viruses lack all of these, **antibiotics have zero effect on viral infections** (colds, flu, COVID-19).",
        "Viral defense relies on **prophylactic vaccines**, **monoclonal antibodies**, and **targeted antivirals** (e.g. `Paxlovid` protease inhibitors, `Remdesivir`/`Acyclovir` polymerase chain terminators).",
        "Bacteriophages ('phages') are specialized viruses that infect only bacteria—and their defense arms race inside bacteria is what evolved **CRISPR-Cas9** in nature!",
    ),
    (
        "Computer Science · Data Structures",
        "Arrays / Vectors vs. Linked Lists & Stacks vs. Queues",
        ["array", "arrays", "vector", "linkedlist", "doublylinked", "stack", "stacks", "queue", "queues", "deque", "lifo", "fifo"],
        ["array vs linked list", "stack vs queue", "linked list", "dynamic array"],
        "Contiguous Dynamic Arrays (`Vec<T>`, `ArrayList`), Linked Lists, Stacks (`LIFO`), and Queues (`FIFO`) form the foundational linear data structures of computer science.",
        "- **Dynamic Array (`Vec<T>`) vs. Linked List**: A dynamic array stores elements contiguously in a single heap allocation, doubling capacity (`2x` geometric growth) when full for **amortized `O(1)` push** and **`O(1)` random index access (`base + i * sizeof(T)`)**. A Linked List scatters nodes across the heap connected by `next`/`prev` pointers (`O(N)` index search, `O(1)` splice given a node pointer).\n- **Stack (`LIFO`) vs. Queue (`FIFO`)**: A **Stack** (*Last-In, First-Out*) pushes and pops from the top in `O(1)` (powering function call stacks, DFS, undo history, and expression parsing). A **Queue** (*First-In, First-Out*, implemented as a circular ring buffer `VecDeque<T>`) enqueues at the tail and dequeues from the head in `O(1)` (powering BFS, task schedulers, and I/O buffers).",
        "Hardware Cache Reality: scanning a contiguous `Vec<i64>` streams `8` elements per `64-byte` L1 cache line with hardware prefetching (`~0.5 ns/elem`), whereas chasing `LinkedList` pointers incurs a `~60 ns` DRAM cache miss on every node (`>50x` slower in practice even when both are `O(N)`!).",
        "Prefer contiguous `Vec<T>` / `VecDeque<T>` (or arena-indexed vectors) over pointer-based `LinkedList` in `99%` of modern software.",
        "Operating system `io_uring` submission and completion queues use fixed-size power-of-two contiguous ring buffers with bitwise index masking (`idx & (cap - 1)`).",
    ),
    (
        "Computer Science · Probabilistic Structures",
        "Bloom Filters, HyperLogLog & Count-Min Sketch",
        ["bloom", "bloomfilter", "hyperloglog", "sketch", "probabilistic", "bitarray", "falsepositive"],
        ["bloom filter", "probabilistic data structure", "count min sketch"],
        "Probabilistic sketch data structures trade a tiny, mathematically bounded error rate for orders-of-magnitude reductions in memory footprint when tracking billions of items.",
        "- **Bloom Filter (Membership Testing)**: Uses an `m`-bit array and `k` independent hash functions. Inserting item `x` sets bits `h_1(x), ..., h_k(x)` to `1`. Querying `x` checks if all `k` bits are `1`: if any bit is `0`, `x` is **100% guaranteed NOT in the set (zero false negatives)**; if all are `1`, `x` is present with false-positive probability `p approx (1 - e^(-k n / m))^k`.\n- **HyperLogLog (Cardinality Estimation)**: Counts the number of distinct elements (`COUNT(DISTINCT)`) in a stream of billions of items using only **`1.5 KB`** of memory (`~1%` standard error) by tracking the maximum number of leading zeros in hashed values.\n- **4096-Bit Manifold Sketch (`R4.DurableMemory`, `R4-26`)**: Gates memory admission in `UOR-R4` at the exact `+3-sigma` threshold of **`2,144 / 4,096` matching bits**.",
        "Optimal Bloom filter parameters for false-positive rate `p`: bits per element `m / n = -ln(p) / (ln 2)^2 approx 1.44 * log2(1/p)` (`~10 bits` per item for `1%` false-positive rate) with `k = 0.693 * (m / n)` hashes.",
        "Standard Bloom filters do not support deletion (turning a bit to `0` could clear another item's bit); **Cuckoo Filters** or **Counting Bloom Filters** add deletion support at `~1.5x-3x` bits per entry.",
        "LSM-Tree databases (`RocksDB`, `Cassandra`) attach a `10-bit/key` Bloom filter to every on-disk SSTable, eliminating `99%` of unnecessary disk reads for non-existent keys.",
    ),
    (
        "Artificial Intelligence · Retrieval & Tokenization",
        "Tokenization (BPE), Embeddings, Vector Databases (HNSW) & RAG",
        ["tokenization", "tokenizer", "bpe", "subword", "embedding", "embeddings", "vectordb", "hnsw", "rag", "cosine", "retrieval"],
        ["byte pair encoding", "vector database", "retrieval augmented generation", "how tokenization works"],
        "Modern language models convert raw UTF-8 text into discrete vocabulary IDs via subword tokenization (`BPE`), project sequences into dense or integer coordinate embeddings, and ground responses in external knowledge via Retrieval-Augmented Generation (`RAG`).",
        "- **Byte-Pair Encoding (`BPE` Tokenizer)**: Starts with the 256 raw byte tokens (`0x00..0xFF`) and iteratively merges the most frequent adjacent pair into a new vocabulary piece until reaching target vocabulary size (`4,096` in `UOR-R4`, `32k–128k` in standard LLMs). Because every byte is in the base alphabet, byte-level BPE is **100% lossless and reversible** with zero `<UNK>` out-of-vocabulary tokens.\n- **Vector Search (`HNSW` vs. `UOR-R4 /8 Point-Key` & `/6 Hamming`)**: Continuous vector DBs index floating-point embeddings using Hierarchical Navigable Small World (`HNSW`) graphs (`O(log N)` approximate greedy walk) or Product Quantization (`IVF-PQ`). `UOR-R4` replaces float cosine similarity with exact integer **`Q12` L1 distance (`memory.point.points`)** and **64-bit SWAR Hamming distance** (`1` CPU cycle per 64 dimensions).",
        "Cosine similarity: `cos(theta) = (u . v) / (||u|| * ||v||)`. In `UOR-R4`, `R4.TokenizerKernel` (`R4-18..R4-19`) proves total deterministic UTF-8 tokenization and canonical JSON serialization.",
        "`RAG` (Retrieval-Augmented Generation) decouples updating factual world knowledge (adding documents to the index in milliseconds) from expensive model retraining, while providing verifiable source citations.",
        "Our bundled `gpu/tokenizer.json` (`4,096` pieces, SHA-256 `d36d3e87...`) implements a number-aware byte-level BPE that splits digits cleanly so multi-digit arithmetic (`R4-27`) generalizes across arbitrary lengths.",
    ),
    (
        "Finance & Investing",
        "Stocks (Equities) vs. Bonds (Fixed Income), Index Funds & Portfolio Diversification",
        ["stocks", "equities", "bonds", "yield", "treasury", "etf", "indexfund", "sp500", "sharpe", "diversification", "dividend"],
        ["stocks vs bonds", "index fund", "portfolio diversification", "modern portfolio theory"],
        "Financial markets allocate capital primarily through **Equities (Stocks)**—representing fractional ownership of corporate cash flows and growth—and **Fixed Income (Bonds)**—representing contractual debt obligations with scheduled coupon interest and principal repayment.",
        "- **Stocks (Equities)**: Shareholders own residual claims on company earnings (`dividends` + `retained growth`). Higher long-run expected real return (`~6.5–7.5%` historical real annualized equity return) paired with higher short-term drawdown volatility.\n- **Bonds (Fixed Income)**: Bondholders lend principal to a government (`US Treasuries`) or corporation in exchange for fixed coupon payments and par repayment at maturity. In the capital structure, bondholders have **senior priority** over shareholders.\n- **Inverse Bond Price–Yield Relationship**: When market interest rates `y` rise, the price `P` of existing lower-coupon bonds falls mathematically (`P = sum C / (1 + y)^t + Par / (1 + y)^T`), with sensitivity measured by **Modified Duration (`Delta P / P approx -D_mod * Delta y`)**.",
        "Sharpe Ratio (risk-adjusted return): `S = (E[R_p] - R_f) / sigma_p`. By **Harry Markowitz's Modern Portfolio Theory**, combining imperfectly correlated assets (`corr(A, B) < 1`) reduces portfolio variance `sigma_p^2` below the weighted average of individual variances—the 'only free lunch in finance'.",
        "Low-cost, broad-market **Index Funds / ETFs** (`expense ratio 0.03%`) outperform `~85–90%` of actively managed high-fee funds over `15+ year` horizons after fees and taxes.",
        "A bond with a `7-year` modified duration will drop `~7%` in market price if prevailing interest rates rise by `+1.0%` (`100 bps`).",
    ),
    (
        "Space Exploration & Planetary Science",
        "Space Exploration: Rocket Propulsion (Tsiolkovsky Equation), Orbital Mechanics, JWST & Europa",
        ["rocket", "spacex", "apollo", "jwst", "mars", "europa", "enceladus", "orbit", "delta_v", "tsiolkovsky", "satellite", "iss"],
        ["rocket equation", "orbital mechanics", "james webb", "life on europa", "space exploration"],
        "Spaceflight and planetary exploration rely on **Tsiolkovsky's Rocket Equation**, **Keplerian/Hohmann orbital mechanics**, and deep-space observatories (`JWST`, `Europa Clipper`) investigating cosmic origins and ocean-world astrobiology.",
        "- **Tsiolkovsky Rocket Equation**: Because a rocket must accelerate its own unburned propellant in vacuum, maximum velocity change `Delta v` scales logarithmically with wet-to-dry mass ratio: **`Delta v = I_sp * g_0 * ln(m_0 / m_f)`** (`I_sp` = specific impulse: `~311s` for RP-1/LOX, `~380s` for Methalox `Raptor`, `~450s` for Hydrolox `RS-25`, `~3,000s` for Hall-effect ion thrusters). Multi-stage rockets discard empty tank mass `m_f` mid-flight to overcome this tyranny.\n- **Orbital Mechanics & Hohmann Transfers**: Reaching Low Earth Orbit (`LEO`, `~400 km`) is about horizontal velocity (**`7.8 km/s` = `28,000 km/h`**), not just altitude! Raising an orbit most efficiently uses a two-burn elliptical **Hohmann Transfer** (prograde burn at periapsis, circularization burn at apoapsis).\n- **JWST & Ocean Worlds (`Europa` / `Enceladus`)**: The **James Webb Space Telescope** orbits the Sun-Earth **`L2` Lagrange point** (`1.5 million km` away), passively cooled to `< 40 K` behind a 5-layer sunshield to observe high-redshift infrared galaxies (`z > 13`). Meanwhile, Jupiter's moon **Europa** and Saturn's **Enceladus** harbor global liquid-water oceans beneath icy shells, kept warm by **tidal flexing** and fed by seafloor hydrothermal vents.",
        "Tsiolkovsky Rocket Equation: `Delta v = v_e * ln(m_initial / m_final)`. Escape velocity from body of mass `M`, radius `R`: `v_esc = sqrt(2 G M / R) = sqrt(2) * v_orbit` (`11.2 km/s` from Earth).",
        "Chemical rockets provide high thrust-to-weight (`T/W > 1`) required to escape planetary gravity wells, while electric/ion propulsion (`Xenon`/`Krypton` Hall thrusters) provides `10x` higher fuel efficiency (`I_sp`) for deep-space interplanetary cruise.",
        "Counterintuitively, in orbital mechanics, firing your thrusters *forward* pushes you into a higher orbit where you actually travel *slower* (`v_orbit = sqrt(G M / r)`); to catch up to a spacecraft ahead of you, you briefly brake into a lower, faster orbit!",
    ),
]

ALL_GLM_CONCEPTS = (
    GLM_CONCEPTS_PART1
    + GLM_CONCEPTS_PART2
    + GLM_CONCEPTS_PART3
    + GLM_CONCEPTS_PART4
    + GLM_CONCEPTS_PART5
)


GLM_CONCEPTS_PART6 = [
    # ==================== ADDITIONAL GRANULAR CONCEPTS FOR DEEP GENERAL COVERAGE ====================
    (
        "Computer Science · Networking",
        "TCP (Transmission Control Protocol) — 3-Way Handshake, Flow Control & Congestion Control",
        ["tcp", "handshake", "syn", "ack", "retransmission", "cubic", "bbr", "slidingwindow"],
        ["tcp handshake", "3 way handshake", "transmission control protocol", "tcp congestion"],
        "TCP is the reliable, connection-oriented byte-stream transport protocol (`RFC 9293`) underlying HTTP/1.1, HTTP/2, SSH, TLS, and database connections.",
        "1. **3-Way Handshake**: Client sends `SYN(seq=x)` -> Server replies `SYN-ACK(seq=y, ack=x+1)` -> Client sends `ACK(ack=y+1)`.\n2. **Reliability & Ordering**: Every byte has a 32-bit sequence number; unacknowledged segments are retransmitted, and out-of-order segments are reassembled before delivery to the application socket.\n3. **Flow & Congestion Control**: Receiver advertises `rwnd` (receive window) to prevent buffer overflow; sender tracks congestion window `cwnd` via Slow Start, Congestion Avoidance, Fast Retransmit (`3 duplicate ACKs`), and `BBR`/`CUBIC` pacing.",
        "Effective Sending Window: `W = min(cwnd, rwnd)`. Throughput bound (Mathis equation): `BW <= (MSS / RTT) * (1 / sqrt(p_loss))`.",
        "Strengths: guaranteed delivery, exact byte ordering, and automatic network congestion adaptation. Trade-offs: `1-RTT` connection setup latency and **Head-of-Line (`HoL`) blocking** when a single lost packet stalls the entire byte stream.",
        "Opening an HTTPS connection over TCP + TLS 1.3 takes `1 RTT` for TCP `SYN/SYN-ACK` plus `1 RTT` for the TLS 1.3 handshake (`2 RTTs` total before first byte).",
    ),
    (
        "Computer Science · Networking",
        "UDP (User Datagram Protocol) & QUIC — Low-Latency Connectionless Transport",
        ["udp", "datagram", "connectionless", "webrtc", "voip", "quic"],
        ["user datagram protocol", "udp protocol", "how udp works"],
        "UDP (`RFC 768`) is a minimal, connectionless transport protocol that sends independent datagrams with an `8-byte` header (`Source Port, Dest Port, Length, Checksum`) and zero handshake or retransmission delay.",
        "Unlike TCP, UDP maintains no connection state in the OS kernel, performs no automatic retransmission of lost packets, and delivers each arriving datagram immediately to the application regardless of whether earlier packets were dropped. Modern **`QUIC` (`HTTP/3`)** layers stream multiplexing, per-stream flow control, and `TLS 1.3` encryption on top of UDP in user space.",
        "UDP Header Overhead: `8 bytes` (vs `20–60 bytes` for TCP), `0-RTT` setup delay, and zero inter-packet Head-of-Line blocking.",
        "Strengths: minimal latency, zero head-of-line blocking, multicast/broadcast support, and full application control over pacing. Trade-offs: applications must handle packet loss, ordering, and congestion control themselves (or via `QUIC`/`WebRTC`).",
        "Powers DNS queries (`port 53`), `HTTP/3` (`QUIC`), live video conferencing (`WebRTC`), multiplayer game state sync, and high-frequency telemetry.",
    ),
    (
        "Systems · Databases",
        "Relational SQL Databases (PostgreSQL, MySQL, SQLite) — Schemas, Joins & ACID",
        ["sql", "relational", "postgres", "postgresql", "mysql", "sqlite", "rdbms", "normalization", "foreignkey"],
        ["relational database", "sql database", "third normal form", "acid compliance"],
        "Relational Database Management Systems (`RDBMS`) organize data into normalized tables (`relations` of rows/tuples and typed columns/attributes) queried declaratively via `SQL` and protected by strict `ACID` transactions.",
        "Data is normalized (`1NF -> 2NF -> 3NF / BCNF`) to eliminate update anomalies and redundant storage, while **Foreign Keys** enforce referential integrity across tables. The query optimizer translates declarative `SELECT ... JOIN ... WHERE ... GROUP BY` queries into physical execution plans (`Index Scan`, `Hash Join`, `Merge Join`) backed by `B+Tree` indexes, `MVCC` snapshot isolation, and a crash-proof Write-Ahead Log (`WAL`).",
        "Relational Algebra operators: Selection (`sigma`), Projection (`pi`), Natural/Equi-Join (`bowtie`), Union (`cup`), and Aggregation (`gamma`).",
        "Strengths: strong data integrity, arbitrary ad-hoc multi-table joins, mature ACID transactions, and declarative indexing. Trade-offs: horizontal write scaling across multiple master nodes is harder than in partitioned NoSQL key-value stores.",
        "`PostgreSQL` combines strict relational ACID guarantees with `JSONB` document indexing, full-text search, and `pgvector` embeddings in a single engine.",
    ),
    (
        "Systems · Databases",
        "NoSQL Databases (Key-Value, Document, Wide-Column & Graph Stores)",
        ["nosql", "mongodb", "dynamodb", "cassandra", "scylladb", "neo4j", "documentdb", "keyvalue"],
        ["nosql database", "document database", "key value store", "wide column store"],
        "NoSQL databases trade relational multi-table joins and rigid schemas for horizontal partition scalability, flexible document/column models, and predictable single-digit-millisecond latency at massive scale.",
        "Categorized into four architectures:\n1. **Key-Value Stores (`Redis`, `DynamoDB`)**: `O(1)` hash-partitioned lookups by primary key.\n2. **Document Stores (`MongoDB`, `Firestore`)**: Store self-contained hierarchical `JSON`/`BSON` documents with secondary indexes.\n3. **Wide-Column Stores (`Cassandra`, `ScyllaDB`, `Bigtable`)**: Partition rows by `(Partition Key, Clustering Key)` over `LSM-Trees` for massive write throughput.\n4. **Graph Databases (`Neo4j`)**: Store nodes and index-free adjacency edges for fast multi-hop relationship traversal.",
        "Partition routing: `Node = ConsistentHash(Partition_Key)`. Tunable quorum consistency: `R + W > N`.",
        "Strengths: effortless horizontal sharding across hundreds of nodes, high write throughput (`LSM`), and schema flexibility. Trade-offs: lack of cross-partition ACID joins requires denormalizing data around known access patterns at design time.",
        "Discord migrated trillions of chat messages to `ScyllaDB` (`LSM` wide-column store partitioned by `(channel_id, bucket)`), achieving predictable sub-15ms tail latency.",
    ),
    (
        "Physics · Foundations",
        "Classical Mechanics vs. Quantum Mechanics",
        ["classical", "newtonian", "macroscopic", "microscopic", "deterministic", "wavefunction", "planck"],
        ["classical vs quantum", "classical mechanics", "ehrenfest theorem", "correspondence principle"],
        "Classical Mechanics governs macroscopic objects (`v << c`, action `S >> hbar`) with deterministic trajectories `(x(t), p(t))`, whereas Quantum Mechanics governs atomic and subatomic scales (`S ~ hbar`) where observables are discrete operators acting on complex probability wavefunctions `|psi>`.",
        "- **State & Evolution**: In Classical Mechanics, a particle has an exact simultaneous position `x` and momentum `p` evolving via Hamilton's equations (`dx/dt = dH/dp`, `dp/dt = -dH/dx`). In Quantum Mechanics, the state is a complex vector `|psi>` in Hilbert space evolving unitarily via `i hbar d|psi>/dt = H |psi>`, and position/momentum obey `[x, p] = i hbar` (`Delta x * Delta p >= hbar / 2`).\n- **Bohr's Correspondence Principle & Decoherence**: As quantum numbers grow large and a system interacts with `10^23` environmental degrees of freedom (**Quantum Decoherence**), off-diagonal interference terms vanish rapidly and quantum expectation values obey Newton's classical laws (**Ehrenfest's Theorem**: `d<x>/dt = <p>/m`, `d<p>/dt = -<grad V>`).",
        "Feynman Path Integral bridge: `K(b, a) = integral D[x(t)] exp(i * S[x(t)] / hbar)`. In the classical limit `hbar -> 0`, stationary phase (`delta S = 0`) recovers Hamilton's Principle of Least Action!",
        "Classical mechanics is exact and computationally tractable for bridges, rockets, orbits, and fluids; Quantum mechanics is required for transistors, lasers, chemical bonding, superconductors, MRI, and nuclear physics.",
        "Without quantum tunneling through the Coulomb barrier, protons in the Sun's core (`1.5 * 10^7 K`) could never fuse and stars would not shine!",
    ),
    (
        "Artificial Intelligence · Paradigms",
        "Supervised vs. Unsupervised vs. Self-Supervised vs. Reinforcement Learning",
        ["supervised", "unsupervised", "selfsupervised", "labeled", "unlabeled", "clustering", "contrastive"],
        ["supervised vs unsupervised", "types of machine learning", "self supervised learning"],
        "Machine learning is organized into four core training paradigms based on the structure of the supervision signal available in the data.",
        "1. **Supervised Learning (`(x_i, y_i)` labeled pairs)**: Learns a mapping `f(x) -> y` by minimizing cross-entropy (classification) or mean-squared error (regression) against ground-truth human or empirical labels (`XGBoost`, `ResNet`, fine-tuned classifiers).\n2. **Unsupervised Learning (`{x_i}` unlabeled data)**: Discovers intrinsic latent structure, clusters (`K-Means`, `DBSCAN`), density modes (`GMM`), or low-dimensional manifolds (`PCA`, `UMAP`) without target labels.\n3. **Self-Supervised Learning (`SSL`, Pretraining)**: Turns massive unlabeled corpora into supervised tasks by predicting masked or future parts of the input itself (next-token prediction in LLMs/GLMs, masked autoencoding, `CLIP` contrastive alignment).\n4. **Reinforcement Learning (`RL`)**: An agent interacts with an environment `(s_t, a_t, r_t, s_{t+1})` to maximize long-term cumulative reward via trial-and-error exploration and credit assignment.",
        "Yann LeCun's Cake Analogy: **Self-Supervised Learning** is the cake (`millions` of bits of predictive feedback per sample), **Supervised Learning** is the icing (`10–1,000` bits per label), and **Reinforcement Learning** is the cherry (`1` scalar reward bit per episode).",
        "Modern frontier AI combines **Self-Supervised Pretraining** (for broad world knowledge) -> **Supervised Fine-Tuning (`SFT`)** (for instruction following) -> **Reinforcement Learning (`RLHF`/`DPO`/verifiable RL)** (for reasoning and alignment).",
        "In `UOR-R4`, self-supervised sequence admission populates `/6..8` lookup tables while coupled coordinate sweeps (`R4.Learning`) refine discrete gate and carrier coordinates.",
    ),
    (
        "World Geography & Demographics",
        "World Continents, Oceans, Major Rivers, Mountain Ranges & Global Geography",
        ["geography", "continent", "continents", "ocean", "oceans", "everest", "himalayas", "amazon", "nile", "pacific", "atlantic", "country", "countries", "population"],
        ["seven continents", "five oceans", "tallest mountain", "longest river", "world geography"],
        "Earth's surface (`510.1 million km^2`) is divided into **`70.8%` interconnected oceans (`5` ocean basins)** and **`29.2%` continental landmass (`7` continents)** shaped by plate tectonics and hydrology.",
        "- **The 7 Continents (by land area & population)**: **Asia** (`44.6M km^2`, `~4.7B` people), **Africa** (`30.4M km^2`, `~1.4B`), **North America** (`24.7M km^2`), **South America** (`17.8M km^2`), **Antarctica** (`14.2M km^2`), **Europe** (`10.2M km^2`), and **Australia / Oceania** (`8.5M km^2`).\n- **The 5 Oceans (by volume & area)**: **Pacific Ocean** (largest and deepest: *Challenger Deep* in the Mariana Trench reaches **`10,935 m`**), **Atlantic Ocean**, **Indian Ocean**, **Southern (Antarctic) Ocean**, and **Arctic Ocean**.\n- **Extreme Landmarks**:\n  - *Highest Peak above Sea Level*: **Mount Everest** (`8,848.86 m` / `29,031.7 ft` in the Himalayas, formed by the India–Eurasia continental collision); *Tallest Base-to-Peak*: **Mauna Kea** (`~10,210 m` from Pacific seafloor).\n  - *Longest & Largest Rivers*: The **Nile** (`~6,650 km` in Northeast Africa) and the **Amazon** (`~6,400–6,992 km`, carrying **`~20%` of all river freshwater on Earth**—more than the next 7 largest rivers combined!).",
        "Hypsometric distribution: average continental elevation is `+840 m` above sea level, while average ocean depth is `-3,688 m`.",
        "Major civilizations historically clustered along navigable alluvial river basins (**Nile, Tigris-Euphrates, Indus, Ganges, Yangtze, Yellow River, Rhine, Danube, Mississippi**) due to fertile silt and low-cost water transport.",
        "Because water has `~4x` the specific heat capacity of rock (`4,184 J/(kg*K)`), maritime currents like the **Gulf Stream** keep Western Europe `~5–10 °C` warmer in winter than inland regions at the same latitude.",
    ),
    (
        "Humanities · Law, Governance & Political Philosophy",
        "Constitutional Law, Separation of Powers, Common Law vs. Civil Law & Democracy",
        ["law", "constitution", "constitutional", "democracy", "republic", "judicial", "legislative", "executive", "commonlaw", "civillaw", "montesquieu", "locke"],
        ["separation of powers", "checks and balances", "common law vs civil law", "rule of law"],
        "Constitutional governance and legal systems institutionalize the **Rule of Law**—ensuring that state authority is bounded by public, predictable laws rather than arbitrary decree.",
        "- **Separation of Powers & Checks and Balances (Locke, Montesquieu, Madison)**: Divides sovereignty across three independent branches: **Legislative** (enacts statutes and appropriates budgets), **Executive** (administers and enforces laws), and **Judicial** (interprets statutes and exercises constitutional review).\n- **Common Law vs. Civil Law**:\n  - **Common Law** (UK, US, Canada, Australia, India): Combines legislative statutes with binding judicial precedent (**`stare decisis`**—like cases must be decided alike), allowing legal doctrine to evolve incrementally through appellate court opinions.\n  - **Civil Law** (Continental Europe, Latin America, East Asia, rooted in Roman *Corpus Juris Civilis* and the Napoleonic/German *BGB* codes): Relies on comprehensive, systematic statutory codes where judges apply codified principles directly without creating binding precedent.",
        "Legal Certainty Invariant: laws must be prospective (no *ex post facto* punishment), publicly promulgated, general, clear, and applied impartially (`Due Process`).",
        "Common Law offers flexible adaptation to novel technologies and commercial practices through case precedent; Civil Law offers compact, unified statutory codification.",
        "In contract law, an enforceable agreement requires **Offer, Acceptance, Consideration (bargained-for exchange), Capacity, and Legal Purpose**.",
    ),
]

ALL_GLM_CONCEPTS = ALL_GLM_CONCEPTS + GLM_CONCEPTS_PART6
