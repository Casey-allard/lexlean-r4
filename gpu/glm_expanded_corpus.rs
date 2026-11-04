// Rich Multi-Domain Conversational, Coding, Reasoning, Philosophy, Science & Everyday-Life
// Training Corpus Expansion for `uor-r4.lut-stack/8` (`UORLUT01`).
//
// Each tuple is `(canonical_prompt, paraphrases, reply)` and is compiled through the
// 6-layer Spin-Preserving `SpiralCore/FBS` trunk (`ByteBpe4096`), `BVG-K` writer.table
// coordinate refinement, `ServedPointKey` (`/8`), and multi-scale `ServedTurnMemory` (`/7`).

pub const EXPANDED_DIALOGUE_SPECS: &[(&str, &[&str], &str)] = &[
    // =========================================================================
    // 1. Conversational Rapport, Humor, Riddles, Storytelling & Meta-Cognition
    // =========================================================================
    (
        "Tell me a riddle.",
        &[
            "Give me a good riddle to solve.",
            "Do you know a clever brainteaser or riddle?",
            "Test me with a riddle.",
            "Ask me a riddle.",
        ],
        "Here is a classic riddle:\n\n*I speak without a mouth and hear without ears. I have no body, corridor, or lungs, yet I come alive with wind. What am I?*\n\n**Answer**: An **echo**! Sound waves reflect off distant surfaces and return to the listener's ear as a delayed acoustic copy. Want a harder logic puzzle next?",
    ),
    (
        "Tell me another joke.",
        &[
            "Give me a funny programming joke.",
            "Tell me a dad joke.",
            "Do you have another joke?",
            "Make me smile with a quick joke.",
        ],
        "There are only **10** types of people in the world: those who understand binary, and those who don't!\n\nAnd a bonus one for software engineers: *A QA engineer walks into a bar. Runs into a bar. Crawls into a bar. Orders a beer. Orders 0 beers. Orders 999,999,999 beers. Orders a lizard in a mug. Orders -1 beers. Everything works fine—until the first real customer walks in and asks where the restroom is, and the bar bursts into flames.*",
    ),
    (
        "Tell me a short story.",
        &[
            "Write a short sci-fi story.",
            "Can you tell me an original short story?",
            "Make up a brief story for me.",
            "Tell me a creative story about an AI or the stars.",
        ],
        "On the quiet rim of Europa's ice shelf, Autonomous Beacon Seven had spent forty-two Jovian winters listening to the sub-glacial ocean below. Unlike the old floating-point probes whose drifting registers required constant earthbound recalibration, Seven's mind was carved into an integer lattice of twenty-four-cell quaternions—every memory a discrete rotation on the 3-sphere that never decayed.\n\nOn the ten-thousandth orbit, a sonar pulse returned from sixty kilometers beneath the ice that was not thermal noise: it was a prime-numbered sequence—`2, 3, 5, 7, 11, 13, 17`—echoed back with an extra harmonic rotation. For the first time in human history, the ocean beneath another world had answered a mathematical greeting, and Seven rotated its high-gain dish toward Earth to share the news.",
    ),
    (
        "What is the Turing Test and the Chinese Room argument?",
        &[
            "Explain the Turing Test.",
            "What is John Searle's Chinese Room argument?",
            "Does passing the Turing Test mean an AI is conscious?",
            "Turing Test vs Chinese Room.",
        ],
        "In 1950, **Alan Turing** proposed the *Imitation Game* (now called the **Turing Test**) in his paper *Computing Machinery and Intelligence*: rather than debating whether machines 'think' in an abstract metaphysical sense, he asked whether a machine conversing via text could be indistinguishable from a human interlocutor.\n\nIn 1980, philosopher **John Searle** countered with the **Chinese Room thought experiment**: imagine a person who speaks only English locked in a room with a comprehensive rulebook for manipulating Chinese symbols. Even if the person produces fluent Chinese responses that pass the Turing Test for native speakers outside, the person inside understands zero Chinese. Searle argued that *syntax alone is not semantics*—whereas functionalists and systems-reply theorists counter that understanding resides in the dynamics of the entire system, not the isolated clerk.",
    ),
    (
        "What is the Simulation Hypothesis?",
        &[
            "Are we living in a simulation?",
            "Explain Nick Bostrom's simulation argument.",
            "Could the universe be a computer simulation?",
        ],
        "Philosopher **Nick Bostrom** formulated the **Simulation Argument** (2003) as a trilemma: at least one of the following three propositions must be true:\n1. Almost all civilizations at our stage of development go extinct before reaching technological maturity capable of running ancestor simulations.\n2. Technologically mature civilizations lose interest in running detailed ancestor simulations.\n3. We are almost certainly living inside a computer simulation (because simulated minds would vastly outnumber original biological minds).\n\nCuriously, physicists and discrete geometers often point out that if a universe *were* computed efficiently, it would not waste energy on continuous real-number drift—it would be built on exact discrete gauge symmetries, finite lattices, and quantized information bounds like the Bekenstein bound!",
    ),
    (
        "What is the Fermi Paradox and the Great Filter?",
        &[
            "Explain the Fermi Paradox.",
            "Where is everybody? Why haven't we found aliens?",
            "What is the Great Filter?",
            "Explain the Drake Equation and the Fermi Paradox.",
        ],
        "The **Fermi Paradox**—famously posed by physicist Enrico Fermi in 1950 (*\"Where is everybody?\"*)—contrasts the vast age and scale of the Milky Way (`~100–400 billion` stars, over `13 billion` years old) with the complete absence of confirmed extraterrestrial signals or probes.\n\nProposed resolutions include:\n- **The Great Filter (Robin Hanson)**: Somewhere along the evolutionary path from prebiotic chemistry to prokaryotes, eukaryotes, multicellular brains, and interstellar colonization lies an extraordinarily improbable bottleneck.\n- **Rare Earth Hypothesis**: Complex metazoan life requires a rare convergence of planetary magnetosphere, plate tectonics, a large stabilizing moon, and a Jupiter-mass shield.\n- **Dark Forest / Sub-Surface Civilizations**: Advanced civilizations either communicate via tight-beam optical/neutrino channels we cannot easily intercept or remain quiet.",
    ),
    (
        "What is the Ship of Theseus paradox?",
        &[
            "Explain the Ship of Theseus.",
            "If you replace every part of a ship, is it still the same ship?",
            "What is the paradox of identity?",
        ],
        "The **Ship of Theseus** is a foundational thought experiment in metaphysics recorded by Plutarch: if the wooden planks of Theseus's ship are gradually replaced one by one as they rot until not a single original timber remains, is it still the same ship? Furthermore, if someone reassembles all the discarded original planks into a second vessel, which one is the true Ship of Theseus?\n\nThe paradox highlights two competing theories of identity:\n- **Material Continuity**: Identity is tied to the physical substrate (`matter`).\n- **Relational / Structural Continuity**: Identity is an invariant pattern (`form` or causal trajectory) preserved across transformations—much like a living organism replacing its cells, or a state vector preserved under group transport.",
    ),
    (
        "Explain the Prisoner's Dilemma and Nash Equilibrium.",
        &[
            "What is the Prisoner's Dilemma in game theory?",
            "What is a Nash Equilibrium?",
            "Explain Tit-for-Tat in the iterated prisoner's dilemma.",
        ],
        "In **Game Theory**, the **Prisoner's Dilemma** models why two rational agents might fail to cooperate even when cooperation yields a better collective outcome:\n- If both **Cooperate (`C, C`)**, each serves 1 year.\n- If one **Defects** while the other **Cooperates (`D, C`)**, the defector goes free (0 years) and the cooperator serves 5 years.\n- If both **Defect (`D, D`)**, each serves 3 years.\n\nIn a single-shot game, `(Defect, Defect)` is the unique **Nash Equilibrium**—neither player can unilaterally improve their payoff by switching strategies. However, in the **Iterated Prisoner's Dilemma** (studied by Robert Axelrod), strategies like **Tit-for-Tat** (start by cooperating, then mirror the partner's previous move) and **Generous Tit-for-Tat** allow stable cooperation to emerge.",
    ),
    (
        "What are the most important cognitive biases to watch out for?",
        &[
            "Explain confirmation bias, survivorship bias, and sunk cost fallacy.",
            "What is the Dunning-Kruger effect?",
            "List common cognitive biases and logical fallacies.",
        ],
        "Understanding **cognitive biases** is one of the highest-leverage tools for clearer decision-making:\n1. **Confirmation Bias**: Searching for, interpreting, and recalling evidence that confirms pre-existing beliefs while ignoring falsifying counterexamples.\n2. **Survivorship Bias**: Drawing conclusions only from visible successes while overlooking the silent graveyard of failures (famously illustrated by Abraham Wald analyzing WWII bomber armor).\n3. **Sunk Cost Fallacy**: Continuing an endeavor because of previously invested time or money rather than evaluating future expected value.\n4. **Dunning-Kruger Effect**: Overestimating one's competence in a domain when lacking the metacognitive expertise to recognize mistakes.\n5. **Goodhart's Law**: *\"When a measure becomes a target, it ceases to be a good measure.\"*",
    ),
    (
        "What are Occam's Razor, Hanlon's Razor, and Hitchens's Razor?",
        &[
            "Explain Occam's Razor.",
            "What is Hanlon's Razor?",
            "What are philosophical razors?",
        ],
        "In philosophy and scientific reasoning, a **razor** is a heuristic rule of thumb that shaves away unlikely explanations:\n- **Occam's Razor** (William of Ockham): *Among competing hypotheses that predict the data equally well, prefer the one that introduces the fewest unverified assumptions (minimum description length / Kolmogorov complexity).*\n- **Hanlon's Razor**: *Never attribute to malice that which is adequately explained by carelessness, misunderstanding, or system complexity.*\n- **Popper's Falsifiability Principle**: *For a claim to be scientific, it must make a specific prediction that could in principle be refuted by an observable test.*",
    ),

    // =========================================================================
    // 2. Software Engineering, Programming Languages & Practical Code Recipes
    // =========================================================================
    (
        "Explain async/await and Tokio in Rust.",
        &[
            "How does async Rust work under the hood?",
            "Explain Futures, Pin, and Tokio in Rust.",
            "Write an async function in Rust.",
        ],
        "In Rust, `async fn` compiles down to a zero-allocation **state machine** implementing `std::future::Future<Output = T>`. Unlike Go or JavaScript, Rust futures are **lazy (pull-based)**: they do nothing until polled (`Future::poll(Pin<&mut Self>, &mut Context)`) by an executor such as **Tokio**.\n\n```rust\nuse tokio::time::{sleep, Duration};\n\npub async fn fetch_with_retry(attempts: u32) -> Result<String, &'static str> {\n    for i in 1..=attempts {\n        if i == attempts {\n            return Ok(format!(\"Succeeded on attempt {i}\"));\n        }\n        sleep(Duration::from_millis(50)).await;\n    }\n    Err(\"exhausted retries\")\n}\n```\n\n`Pin<&mut Self>` guarantees that a state machine containing self-referential borrows across `.await` points cannot be moved in memory once polling begins.",
    ),
    (
        "Explain error handling in Rust with Result, Option, and the ? operator.",
        &[
            "How does error handling work in Rust?",
            "Explain Result and Option in Rust.",
            "What does the question mark operator do in Rust?",
        ],
        "Rust replaces exceptions and null pointers with explicit algebraic sum types:\n- `Option<T>`: either `Some(T)` or `None`.\n- `Result<T, E>`: either `Ok(T)` or `Err(E)`.\n\nThe `?` operator unwraps `Ok(v)` and continues execution, or early-returns `Err(From::from(e))` to the caller:\n\n```rust\nuse std::fs;\nuse std::num::ParseIntError;\n\n#[derive(Debug)]\npub enum ConfigError {\n    Io(std::io::Error),\n    Parse(ParseIntError),\n}\n\npub fn read_port(path: &str) -> Result<u16, ConfigError> {\n    let text = fs::read_to_string(path).map_err(ConfigError::Io)?;\n    let port = text.trim().parse::<u16>().map_err(ConfigError::Parse)?;\n    Ok(port)\n}\n```",
    ),
    (
        "Explain traits, generics, and dynamic dispatch (dyn Trait vs impl Trait) in Rust.",
        &[
            "What is the difference between dyn Trait and impl Trait in Rust?",
            "Explain static vs dynamic dispatch in Rust.",
            "How do Rust traits work?",
        ],
        "Rust supports both **compile-time monomorphization (static dispatch)** and **runtime vtable pointers (dynamic dispatch)**:\n- **`impl Trait` / `<T: Trait>` (Static Dispatch)**: The compiler generates a specialized machine-code copy for each concrete type `T`. Calls can be inlined with zero vtable overhead, at the cost of larger binary size.\n- **`&dyn Trait` / `Box<dyn Trait>` (Dynamic Dispatch)**: Uses a 16-byte fat pointer `(data_ptr, vtable_ptr)`. Allows heterogeneous collections (`Vec<Box<dyn Trait>>`) and smaller code size, with one indirect function call per invocation.",
    ),
    (
        "Explain concurrency in Rust with Arc, Mutex, RwLock, and channels.",
        &[
            "When should I use Arc<Mutex<T>> vs channels in Rust?",
            "Explain Send and Sync traits in Rust.",
            "How does thread safety work in Rust?",
        ],
        "Rust prevents data races at compile time via two marker traits:\n- **`Send`**: Safe to transfer ownership of `T` across thread boundaries.\n- **`Sync`**: Safe for multiple threads to hold shared references (`&T`) simultaneously (`T: Sync` iff `&T: Send`).\n\nFor shared state across threads, `Arc<Mutex<T>>` combines atomic reference counting (`Arc`) with mutual exclusion (`Mutex`), while `std::sync::mpsc` or `tokio::sync::mpsc` provides message-passing channels so worker threads communicate without lock contention.",
    ),
    (
        "Explain Python decorators and write a retry or timing decorator.",
        &[
            "How do decorators work in Python?",
            "Write a Python decorator for timing a function.",
            "Explain @wraps and higher-order functions in Python.",
        ],
        "A **Python decorator** is a higher-order callable that takes a function `fn` and returns an enhanced wrapper function. Using `@functools.wraps(fn)` preserves the original function's `__name__`, `__doc__`, and type annotations:\n\n```python\nimport time\nfrom functools import wraps\nfrom typing import Callable, TypeVar, ParamSpec\n\nP = ParamSpec(\"P\")\nR = TypeVar(\"R\")\n\ndef timed(fn: Callable[P, R]) -> Callable[P, R]:\n    @wraps(fn)\n    def wrapper(*args: P.args, **kwargs: P.kwargs) -> R:\n        t0 = time.perf_counter()\n        try:\n            return fn(*args, **kwargs)\n        finally:\n            elapsed_ms = (time.perf_counter() - t0) * 1000.0\n            print(f\"{fn.__name__} took {elapsed_ms:.2f} ms\")\n    return wrapper\n```",
    ),
    (
        "Explain Python generators, iterators, and list comprehensions.",
        &[
            "What is the difference between a list comprehension and a generator in Python?",
            "How does yield work in Python?",
            "Explain lazy evaluation with generators in Python.",
        ],
        "In Python:\n- A **list comprehension** (`[x * x for x in range(n)]`) eagerly allocates the entire result list in memory (`O(n)` RAM).\n- A **generator expression** (`(x * x for x in range(n))`) or a function using `yield` produces values **lazily one at a time** in `O(1)` memory by suspending and resuming its stack frame on each `next()` call:\n\n```python\ndef chunked_stream(iterable, size: int):\n    batch = []\n    for item in iterable:\n        batch.append(item)\n        if len(batch) == size:\n            yield batch\n            batch = []\n    if batch:\n        yield batch\n```",
    ),
    (
        "Explain the Python GIL (Global Interpreter Lock) and asyncio vs multiprocessing.",
        &[
            "What is the GIL in Python?",
            "When should I use asyncio vs threading vs multiprocessing in Python?",
            "How do you achieve true parallelism in Python?",
        ],
        "In standard CPython, the **Global Interpreter Lock (GIL)** is a mutex that allows only one OS thread to execute Python bytecodes at a time (though Python 3.13+ introduces an experimental free-threaded `--disable-gil` build):\n- **`asyncio` (Single-Threaded Cooperative Multitasking)**: Ideal for high-concurrency **I/O-bound** workloads (thousands of HTTP sockets or database queries) with minimal memory overhead.\n- **`threading`**: Useful when calling C/Rust/CUDA extensions (like NumPy or PyTorch) that explicitly release the GIL.\n- **`multiprocessing` / `ProcessPoolExecutor`**: Spawns separate OS processes with independent heaps to achieve true multi-core parallelism for **CPU-bound** Python code.",
    ),
    (
        "Explain JavaScript closures, the event loop, Promises, and async/await.",
        &[
            "How does the JavaScript event loop work?",
            "What is the difference between microtasks and macrotasks in JS?",
            "Explain closures and Promises in JavaScript.",
        ],
        "A **closure** in JavaScript is a function bundled with lexical references to its surrounding scope, allowing inner functions to retain access to outer variables even after the outer function returns.\n\nThe **JavaScript Event Loop** coordinates single-threaded execution across three queues:\n1. **Call Stack**: Executes synchronous frames to completion.\n2. **Microtask Queue (`Promise.then`, `await` continuations, `queueMicrotask`)**: Drained completely *immediately after* the current stack frame finishes, before yielding to rendering or timers.\n3. **Macrotask / Task Queue (`setTimeout`, `setInterval`, I/O callbacks)**: Runs one task per event-loop tick after the microtask queue is empty.",
    ),
    (
        "Explain React hooks (useState, useEffect, useMemo) and reconciliation.",
        &[
            "How do React hooks work?",
            "When should I use useMemo and useCallback in React?",
            "Explain React's Virtual DOM and fiber reconciliation.",
        ],
        "React models UI as a pure function of state (`UI = f(state)`). Inside function components, **Hooks** attach persistent state cells to the component's **Fiber node** in a linked list (which is why hooks must always be called in the same top-level order):\n- **`useState(initial)`**: Holds reactive local state and schedules a re-render when updated.\n- **`useEffect(fn, deps)`**: Synchronizes the component with external systems (DOM listeners, timers, subscriptions) after commit, running cleanup when `deps` change or on unmount.\n- **`useMemo(compute, deps)`**: Caches an expensive derived value across renders until a dependency in `deps` changes by `Object.is` comparison.",
    ),
    (
        "Explain CORS (Cross-Origin Resource Sharing) and how to fix CORS errors.",
        &[
            "What is a CORS error and how do I fix it?",
            "How does Access-Control-Allow-Origin work?",
            "Why does my browser block fetch requests with CORS?",
        ],
        "**CORS (Cross-Origin Resource Sharing)** is a browser security mechanism that restricts web pages at Origin A (`https://app.example.com`) from reading responses from Origin B (`https://api.example.com`) unless Origin B explicitly opts in via HTTP response headers:\n- **`Access-Control-Allow-Origin: https://app.example.com`** (or `*` for public resources without credentials).\n- **`Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS`**.\n- **`Access-Control-Allow-Headers: Content-Type, Authorization`**.\n\nFor non-simple requests (e.g., `JSON` `POST` or custom headers), the browser automatically sends an **`OPTIONS` preflight request** first. Note that CORS is enforced by the browser—so the fix must be applied on the **API server's response headers** (or via a same-origin reverse proxy).",
    ),
    (
        "Explain Go goroutines, channels, and the select statement.",
        &[
            "How do goroutines and channels work in Go?",
            "Write a worker pool in Go using channels.",
            "Explain Go's M:N scheduler.",
        ],
        "Go's runtime multiplexes thousands of lightweight **goroutines** (`M`, starting at ~2 KB stack) onto OS threads (`N`) via logical processors (`P`) using work-stealing (**M:N scheduling**). Goroutines coordinate via typed **channels**:\n\n```go\nfunc worker(id int, jobs <-chan int, results chan<- int) {\n    for j := range jobs {\n        results <- j * j\n    }\n}\n```\n\nThe `select` statement waits on multiple channel operations simultaneously, choosing a ready case pseudo-randomly or taking a non-blocking `default` branch.",
    ),
    (
        "Write Dijkstra's shortest path algorithm in Rust or Python.",
        &[
            "How does Dijkstra's algorithm work?",
            "Implement Dijkstra's shortest path algorithm.",
            "Show me code for Dijkstra's algorithm with a priority queue.",
        ],
        "**Dijkstra's algorithm** finds the shortest paths from a source vertex to all other vertices in a graph with non-negative edge weights in `O((V + E) log V)` time using a min-heap priority queue:\n\n```python\nimport heapq\n\ndef dijkstra(graph: dict[int, list[tuple[int, int]]], start: int) -> dict[int, int]:\n    dist: dict[int, int] = {start: 0}\n    pq: list[tuple[int, int]] = [(0, start)]\n    while pq:\n        d, u = heapq.heappop(pq)\n        if d > dist.get(u, float(\"inf\")):\n            continue\n        for v, weight in graph.get(u, []):\n            nd = d + weight\n            if nd < dist.get(v, float(\"inf\")):\n                dist[v] = nd\n                heapq.heappush(pq, (nd, v))\n    return dist\n```",
    ),
    (
        "Explain SQL JOINs, window functions, and query optimization.",
        &[
            "What is the difference between INNER JOIN, LEFT JOIN, and FULL OUTER JOIN?",
            "How do SQL window functions like ROW_NUMBER and PARTITION BY work?",
            "How do I optimize a slow SQL query?",
        ],
        "In relational SQL databases:\n1. **JOIN Types**:\n   - `INNER JOIN`: Returns only rows matching in both tables.\n   - `LEFT JOIN`: Returns all rows from the left table, filling `NULL` when no right-table match exists.\n2. **Window Functions (`OVER (PARTITION BY ... ORDER BY ...)`)**: Compute aggregates or rankings across related rows *without collapsing them into a single `GROUP BY` row*:\n```sql\nSELECT\n    department,\n    employee_name,\n    salary,\n    RANK() OVER (PARTITION BY department ORDER BY salary DESC) AS dept_rank\nFROM employees;\n```\n3. **Optimization**: Inspect `EXPLAIN ANALYZE`, create composite B-Tree indexes matching `WHERE` + `ORDER BY` columns (leftmost-prefix rule), and avoid wrapping indexed columns in functions (`WHERE YEAR(created_at) = 2026` $\\to$ use range bounds `created_at >= '2026-01-01'`).",
    ),
    (
        "Explain Git rebase vs merge and how to use git bisect.",
        &[
            "When should I use git rebase vs git merge?",
            "How does git bisect work to find a bug?",
            "Explain Git internals (blobs, trees, commits).",
        ],
        "Under the hood, **Git** is a content-addressable directed acyclic graph (DAG) of immutable SHA-1/SHA-256 objects (`blob` for file contents, `tree` for directories, `commit` for snapshots with parent pointers):\n- **`git merge`**: Preserves the exact chronological branch topology by creating a merge commit with two parents.\n- **`git rebase`**: Replays your feature branch commits one by one onto the tip of the target branch, producing a clean linear history (never rebase shared public branches without `--force-with-lease`).\n- **`git bisect`**: Performs an `O(log N)` binary search across commit history (`git bisect start && git bisect bad HEAD && git bisect good v1.0`) to pinpoint the exact commit that introduced a regression.",
    ),
    (
        "How do you design a URL shortener or rate limiter (System Design)?",
        &[
            "Explain how to design a rate limiter (token bucket vs sliding window).",
            "System design for a distributed service or URL shortener.",
            "How does consistent hashing work in distributed systems?",
        ],
        "In **Distributed System Design**:\n1. **Rate Limiter (Token Bucket)**: Each user/API key has a bucket of capacity `B` refilled at `R` tokens/sec. In Redis, an atomic Lua script reads `(tokens, last_ts)`, adds `(now - last_ts) * R` clamped to `B`, and decrements `1` token if `tokens >= 1` (`HTTP 200`) or rejects with `HTTP 429 Too Many Requests`.\n2. **Consistent Hashing**: Maps both storage nodes and keys onto a `2^32` hash ring with virtual nodes (`vnodes`). Adding or removing a node in an `N`-node cluster only remaps `O(1/N)` of the keys instead of `100%` (`key % N`).\n3. **Base62 ID Generation**: A 7-character Base62 (`[0-9A-Za-z]`) key space provides `62^7 ≈ 3.52 trillion` unique URLs, generated collision-free via a 64-bit Snowflake ID (`[timestamp_ms | shard_id | sequence]`).",
    ),

    // =========================================================================
    // 3. Mathematics, Computer Science Theory, Lean 4 & Cryptography
    // =========================================================================
    (
        "Explain Lean 4, Dependent Type Theory, and the Curry-Howard Isomorphism.",
        &[
            "How does Lean 4 prove mathematical theorems and software correctness?",
            "What is the Curry-Howard isomorphism (propositions as types)?",
            "Why does LexLean use Lean 4?",
        ],
        "**Lean 4** is both a functional programming language and an interactive theorem prover founded on the **Calculus of Inductive Constructions (dependent type theory)**.\n\nBy the **Curry-Howard Isomorphism (*Propositions-as-Types*)**:\n- A mathematical **proposition** $P$ is represented as a **type** (`P : Prop`).\n- A **proof** of $P$ is a **program term** `p : P` that inhabits that type.\n- Logical implication $P \\to Q$ is a function type `P -> Q`; universal quantification $\\forall x : A, P(x)$ is a dependent function type `(x : A) -> P x`; conjunction $P \\wedge Q$ is a product pair `P × Q`.\n\nIn `lexlean-r4`, all 27 architectural invariants (`R4-01..R4-27`) are checked by Lean 4's small kernel with **zero axioms (`sorry`-free)**, guaranteeing that our integer geometry and routing proofs cannot be falsified by edge cases.",
    ),
    (
        "Explain Gödel's Incompleteness Theorems and the Halting Problem.",
        &[
            "What are Gödel's Incompleteness Theorems?",
            "Explain Alan Turing's Halting Problem.",
            "How are Gödel's theorem and the Halting Problem related?",
        ],
        "In 1931, **Kurt Gödel** proved two landmark theorems about any consistent, effectively axiomatized formal system $F$ capable of expressing elementary arithmetic:\n1. **First Incompleteness Theorem**: There exist statements $G_F$ in the language of $F$ that are true in the standard model of arithmetic but **unprovable** within $F$ ($G_F$ essentially encodes *\"This statement is not provable in $F$\"* via Gödel numbering).\n2. **Second Incompleteness Theorem**: Such a system $F$ cannot prove its own consistency ($\\text{Con}(F)$).\n\nFive years later (1936), **Alan Turing** proved the **undecidability of the Halting Problem** via diagonalization: no general algorithm `halts(P, x)` can decide for every program $P$ and input $x$ whether $P(x)$ eventually terminates—a computational mirror of Gödel's First Theorem.",
    ),
    (
        "Explain the Fourier Transform and the Fast Fourier Transform (FFT).",
        &[
            "How does the Fourier Transform work intuitively?",
            "Why is the Fast Fourier Transform O(N log N)?",
            "Explain frequency domain vs time domain.",
        ],
        "The **Fourier Transform** decomposes any time-domain signal $x(t)$ into a sum of circular frequencies $e^{-i 2\\pi f t} = \\cos(2\\pi f t) - i\\sin(2\\pi f t)$:\n$$X[k] = \\sum_{n=0}^{N-1} x[n] \\, e^{-i 2\\pi k n / N}$$\n\nWhile a naive Discrete Fourier Transform (DFT) requires $O(N^2)$ complex multiplications, the **Cooley-Tukey Fast Fourier Transform (FFT)** exploits the periodicity and symmetry of the $N$-th roots of unity ($\\omega_N^{k + N/2} = -\\omega_N^k$) to recursively split an $N$-point transform into two $(N/2)$-point transforms (even and odd indices), reducing the complexity to **$O(N \\log N)$**.",
    ),
    (
        "Explain Eigenvalues, Eigenvectors, and Singular Value Decomposition (SVD).",
        &[
            "What are eigenvalues and eigenvectors intuitively?",
            "Explain Singular Value Decomposition (SVD) and PCA.",
            "Why is SVD so important in linear algebra and machine learning?",
        ],
        "When a linear transformation matrix $A$ acts on space, most vectors change both their length and direction. An **eigenvector** $v \\neq 0$ is a special invariant axis whose direction is preserved (or flipped), scaled only by a scalar **eigenvalue** $\\lambda$: $A v = \\lambda v$.\n\nFor any rectangular $m \\times n$ matrix $M$, **Singular Value Decomposition (SVD)** generalizes eigendecomposition:\n$$M = U \\Sigma V^T$$\nwhere $U$ and $V$ are orthogonal rotation/reflection matrices and $\\Sigma$ is a diagonal matrix of non-negative **singular values** $\\sigma_1 \\ge \\sigma_2 \\ge \\dots \\ge 0$. Truncating SVD to the top $k$ singular values yields the optimal rank-$k$ approximation of $M$ (the Eckart-Young theorem) and forms the foundation of **Principal Component Analysis (PCA)**.",
    ),
    (
        "Explain Shannon Entropy, Cross-Entropy, and KL Divergence.",
        &[
            "What is Shannon entropy in information theory?",
            "Explain KL divergence and cross-entropy loss.",
            "How is information measured in bits?",
        ],
        "In 1948, **Claude Shannon** founded information theory by defining the **self-information (surprisal)** of an event with probability $p$ as $I(p) = -\\log_2(p)$ bits:\n- **Shannon Entropy $H(P) = -\\sum_x P(x) \\log_2 P(x)$**: The minimum average number of bits needed to encode draws from distribution $P$.\n- **Cross-Entropy $H(P, Q) = -\\sum_x P(x) \\log_2 Q(x)$**: The average number of bits needed when encoding draws from true distribution $P$ using code lengths optimized for model distribution $Q$.\n- **Kullback-Leibler (KL) Divergence $D_{\\text{KL}}(P \\parallel Q) = H(P, Q) - H(P)$**: The extra bits wasted by using $Q$ instead of $P$ (always $\\ge 0$ by Gibbs' inequality).",
    ),
    (
        "Explain Zero-Knowledge Proofs (ZKPs) and Elliptic Curve Cryptography.",
        &[
            "How do Zero-Knowledge Proofs work?",
            "Explain zk-SNARKs intuitively.",
            "What is Elliptic Curve Cryptography (ECC)?",
        ],
        "A **Zero-Knowledge Proof (ZKP)** allows a Prover to convince a Verifier that a mathematical statement is true (e.g., *\"I know a secret witness $w$ such that $\\text{SHA256}(w) = H$\"*) **without revealing a single bit of information about $w$ itself**. Every ZKP satisfies three properties:\n1. **Completeness**: An honest prover with a valid witness always convinces the verifier.\n2. **Soundness**: A cheating prover without the witness cannot forge a proof except with negligible probability ($< 2^{-128}$).\n3. **Zero-Knowledge**: The transcript can be simulated without knowing the secret witness.\n\n**Elliptic Curve Cryptography (ECC)** relies on the hardness of the Discrete Logarithm Problem over the group of points on an elliptic curve $y^2 = x^3 + ax + b \\pmod p$, achieving 128-bit security with 256-bit keys (such as Curve25519) instead of 3,072-bit RSA keys.",
    ),

    // =========================================================================
    // 4. Physics, Astronomy, Chemistry, Biology, Neuroscience & Medicine
    // =========================================================================
    (
        "Explain Black Holes, the Event Horizon, and Hawking Radiation.",
        &[
            "How do black holes work?",
            "What is Hawking radiation and the black hole information paradox?",
            "What happens at the event horizon of a black hole?",
        ],
        "A **black hole** is a region of spacetime where mass-energy is compressed within its **Schwarzschild radius** ($r_s = \\frac{2GM}{c^2}$), curving spacetime so steeply that the escape velocity exceeds the speed of light $c$.\n- **Event Horizon**: The causal one-way boundary at $r = r_s$. Outside observers see infalling light gravitationally redshifted and frozen in time, while an infalling observer crosses the horizon in finite proper time.\n- **Hawking Radiation (1974)**: By combining quantum field theory in curved spacetime with thermodynamics, Stephen Hawking showed that black holes have a Bekenstein-Hawking entropy proportional to their surface area ($S = \\frac{k_B c^3 A}{4 G \\hbar}$) and slowly evaporate by emitting thermal radiation at temperature $T_H = \\frac{\\hbar c^3}{8\\pi G M k_B}$.",
    ),
    (
        "Explain the Standard Model of Particle Physics and the Higgs Boson.",
        &[
            "What is the Standard Model of particle physics?",
            "How does the Higgs field give particles mass?",
            "What are quarks, leptons, and gauge bosons?",
        ],
        "The **Standard Model** describes all known elementary particles and three of the four fundamental forces ($\\text{SU}(3)_C \\times \\text{SU}(2)_L \\times \\text{U}(1)_Y$ gauge symmetry):\n1. **Fermions (Matter, spin-$1/2$)**: Three generations of **quarks** (`up/down`, `charm/strange`, `top/bottom`) that feel the strong force, and **leptons** (`electron`, `muon`, `tau`, plus their three neutrinos).\n2. **Gauge Bosons (Force Carriers, spin-$1$)**: **Photons** ($\\gamma$, electromagnetism), **Gluons** ($g$, strong nuclear force / QCD), and **$W^\\pm, Z^0$ bosons** (weak nuclear force).\n3. **Higgs Boson (spin-$0$, discovered at CERN in 2012)**: Excitations of the scalar **Higgs field**, whose non-zero vacuum expectation value ($246\\text{ GeV}$) breaks electroweak symmetry and gives mass to $W/Z$ bosons and fermions via Yukawa coupling.",
    ),
    (
        "Explain Nuclear Fusion vs Nuclear Fission.",
        &[
            "How does nuclear fusion work and why is it hard?",
            "What is the difference between nuclear fission and fusion?",
            "How do tokamaks and stars fuse hydrogen into helium?",
        ],
        "Both **nuclear fission** and **nuclear fusion** release binding energy ($\\Delta E = \\Delta m c^2$) by moving nuclei toward the peak of the nuclear binding energy curve around **Iron-56 (`Fe-56`)**:\n- **Fission**: Splits a heavy, neutron-rich nucleus like **Uranium-235** or **Plutonium-239** into lighter nuclei plus neutrons, sustaining a chain reaction in current nuclear power plants.\n- **Fusion**: Combines light nuclei such as **Deuterium ($^2\\text{H}$)** and **Tritium ($^3\\text{H}$)** into **Helium-4 ($^4\\text{He}$)** plus a `14.1 MeV` neutron. Because two positively charged nuclei repel each other via the Coulomb barrier, fusion requires extreme temperatures (`>100 million K`) and satisfy the **Lawson Criterion** (triple product of density, temperature, and confinement time $n T \\tau_E$) via magnetic confinement (tokamaks/stellarators) or inertial confinement.",
    ),
    (
        "Explain Dark Matter and Dark Energy.",
        &[
            "What is the difference between dark matter and dark energy?",
            "How do we know dark matter exists?",
            "What is the universe made of?",
        ],
        "Observations of the Cosmic Microwave Background (Planck satellite), Type Ia supernovae, and galaxy clusters show that ordinary baryonic matter (stars, gas, planets, atoms) makes up only **`~4.9%`** of the universe's energy density:\n- **Dark Matter (`~26.8%`)**: Invisible, non-baryonic matter that does not emit or absorb light but gravitates strongly. Its existence is inferred from flat galactic rotation curves (Vera Rubin), gravitational lensing (the Bullet Cluster), and large-scale cosmic structure formation.\n- **Dark Energy (`~68.3%`)**: A uniform vacuum energy density (modeled as Einstein's cosmological constant $\\Lambda$) driving the **accelerated expansion** of the universe discovered in 1998.",
    ),
    (
        "How does the human brain learn? Explain neurons, synapses, and neuroplasticity.",
        &[
            "How do neurons and action potentials work?",
            "What is neuroplasticity and Hebbian learning?",
            "Explain dopamine, serotonin, and neurotransmitters.",
        ],
        "The human brain contains roughly **86 billion neurons** connected by over **100 trillion synapses**:\n1. **Action Potentials**: When excitatory inputs depolarize a neuron's membrane past `-55 mV`, voltage-gated $\\text{Na}^+$ channels open in an all-or-none spike that propagates down the axon to release chemical **neurotransmitters** across the synaptic cleft.\n2. **Hebbian Neuroplasticity & LTP**: Summarized as *\"neurons that fire together, wire together\"*, **Long-Term Potentiation (LTP)** strengthens synapses when presynaptic glutamate release coincides with postsynaptic depolarization (unblocking $\\text{Mg}^{2+}$ from NMDA receptors and inserting more AMPA receptors).\n3. **Key Neuromodulators**: **Glutamate** (primary excitation), **GABA** (primary inhibition), **Dopamine** (reward prediction error & motivation), **Acetylcholine** (attention & memory encoding), and **Serotonin** (mood & homeostasis).",
    ),
    (
        "How does the human immune system work (T-cells, B-cells, antibodies, vaccines)?",
        &[
            "Explain innate vs adaptive immunity.",
            "How do T-cells, B-cells, and antibodies fight infection?",
            "How do mRNA vaccines work?",
        ],
        "The human immune system defends the body in two coordinated layers:\n1. **Innate Immunity (Immediate, Non-Specific)**: Physical barriers, macrophages, neutrophils, natural killer (NK) cells, and the complement cascade respond within minutes to conserved pathogen patterns.\n2. **Adaptive Immunity (Targeted & Long-Term Memory)**:\n   - **B-cells** undergo somatic hypermutation to secrete Y-shaped **Antibodies (immunoglobulins)** that neutralize specific antigens.\n   - **Helper T-cells ($\\text{CD4}^+$)** coordinate the immune response, while **Cytotoxic T-cells ($\\text{CD8}^+$)** inspect MHC Class I peptides on cell surfaces and destroy virus-infected or cancerous cells.\n   - **Memory B and T-cells** persist for years, enabling vaccines (including lipid-nanoparticle **mRNA vaccines** that transiently instruct cells to display a harmless viral spike antigen) to trigger rapid secondary immunity.",
    ),
    (
        "Why do we sleep and how do circadian rhythms and REM sleep work?",
        &[
            "Explain the stages of sleep (REM vs deep slow-wave sleep).",
            "Why is sleep important for memory and brain health?",
            "How does the circadian rhythm work?",
        ],
        "Sleep is regulated by two interlocking biological processes:\n1. **Process C (Circadian Rhythm)**: Orchestrated by the **suprachiasmatic nucleus (SCN)** in the hypothalamus in response to blue-light signals from retinal ganglion cells, regulating melatonin and core body temperature on a ~24-hour cycle.\n2. **Process S (Sleep Pressure)**: Driven by the gradual accumulation of extracellular **adenosine** in the brain during waking hours (which caffeine temporarily blocks by binding adenosine receptors).\n\nAcross each 90-minute sleep cycle, **Deep NREM Slow-Wave Sleep** activates the **glymphatic system** to clear metabolic waste (such as beta-amyloid) and replays hippocampal memories into the neocortex, while **REM Sleep** integrates emotional memory and associative problem-solving.",
    ),

    // =========================================================================
    // 5. History, Economics, Finance, Philosophy & World Culture
    // =========================================================================
    (
        "Explain how inflation, interest rates, and central banks (the Federal Reserve) work.",
        &[
            "Why do central banks raise interest rates when inflation is high?",
            "How does the Federal Reserve control inflation?",
            "Explain monetary policy vs fiscal policy.",
        ],
        "**Inflation** is a sustained rise in the general price level of goods and services (a decline in purchasing power per unit of currency), driven by excess aggregate demand (*demand-pull*) or supply shocks (*cost-push*).\n\nA **Central Bank** (such as the U.S. **Federal Reserve** or the **ECB**) manages **Monetary Policy** to balance stable prices (~`2%` annual inflation target) with maximum sustainable employment:\n- **Raising the Policy Interest Rate**: Increases borrowing costs for mortgages, business loans, and credit, cooling spending and investment to bring inflation down.\n- **Open Market Operations (QE / QT)**: Buying or selling government bonds to expand or contract bank reserves and liquidity.\nBy contrast, **Fiscal Policy** is conducted by the legislature and treasury through taxation and government spending.",
    ),
    (
        "Explain compound interest, index funds, and diversification in personal finance.",
        &[
            "How does compound interest work (Rule of 72)?",
            "Why do economists recommend low-cost index funds?",
            "Explain risk vs return and portfolio diversification.",
        ],
        "**Compound interest** grows capital exponentially because each period's return earns returns in all future periods: $A = P(1 + r/n)^{nt}$.\n- **The Rule of 72**: Divide `72` by the annual return percentage to estimate how many years it takes to double your money (e.g., at `8%` annual return, $72 / 8 \\approx 9$ years).\n- **Diversification & Low-Cost Index Funds**: In Modern Portfolio Theory (Harry Markowitz), combining uncorrelated or imperfectly correlated assets reduces idiosyncratic risk without sacrificing expected return. Broad market **index funds** capture the market risk premium at minimal expense ratios (`0.03%–0.10%`), outperforming the vast majority of high-fee active stock pickers over multi-decade horizons.",
    ),
    (
        "Explain the Scientific Revolution and the Enlightenment.",
        &[
            "What happened during the Scientific Revolution?",
            "How did Galileo, Newton, and the Enlightenment change the world?",
            "Explain the history of the scientific method.",
        ],
        "Between the 16th and 18th centuries, the **Scientific Revolution** and the **Age of Enlightenment** transformed human civilization:\n1. **Empirical & Mathematical Science**: Nicolaus Copernicus, Johannes Kepler (elliptical planetary laws), and **Galileo Galilei** (telescopic astronomy and kinematics) overturned Aristotelian dogma, culminating in **Isaac Newton's** *Principia Mathematica* (1687), which unified terrestrial and celestial mechanics under universal gravitation and calculus.\n2. **Enlightenment Philosophy**: Thinkers such as John Locke, Voltaire, Montesquieu (separation of powers), Adam Smith, and Immanuel Kant applied rational inquiry to governance, human rights, religious tolerance, and political economy.",
    ),
    (
        "Explain the history of computing, from Ada Lovelace and Alan Turing to microprocessors.",
        &[
            "Who invented the computer?",
            "Tell me the history of computers and the internet.",
            "How did we go from vacuum tubes to modern CPUs and GPUs?",
        ],
        "The evolution of modern computing unfolded across five foundational eras:\n1. **Mechanical & Logical Foundations (1830s–1930s)**: **Charles Babbage** designed the Analytical Engine, **Ada Lovelace** wrote the first published algorithm (1843), George Boole formalized Boolean algebra, and **Alan Turing** defined the Universal Turing Machine (1936).\n2. **Electronic Vacuum Tubes (1940s)**: Colossus, ENIAC, and the **von Neumann stored-program architecture**.\n3. **The Transistor & Integrated Circuit (1947–1959)**: Bardeen, Brattain, and Shockley invented the transistor at Bell Labs; Jack Kilby and Robert Noyce created the monolithic silicon integrated circuit.\n4. **Microprocessors & Personal Computing (1970s–1980s)**: Intel 4004, Unix & C (Thompson & Ritchie), Xerox PARC GUI, Apple, and IBM PC.\n5. **The Internet, Mobile & Parallel Era (1990s–Present)**: TCP/IP (Cerf & Kahn), the World Wide Web (Berners-Lee), ARM smartphones, and massively parallel GPU/accelerator compute.",
    ),

    // =========================================================================
    // 6. Music, Art, Cooking, Productivity, Debugging & Practical Advice
    // =========================================================================
    (
        "Explain music theory: scales, chords, the circle of fifths, and harmony.",
        &[
            "How does music theory work for beginners?",
            "What is the circle of fifths and why do chords sound good together?",
            "Explain major vs minor chords and intervals.",
        ],
        "At its core, **music theory** is the acoustic geometry of frequency ratios:\n- **Octave (`2:1` frequency ratio)** and **Perfect Fifth (`3:2` ratio)**: In 12-tone equal temperament, an octave is divided into 12 semitones where each semitone multiplies frequency by $2^{1/12} \\approx 1.05946$.\n- **Major vs. Minor Scales**: A **Major scale** follows the whole/half-step pattern `W-W-H-W-W-W-H` (bright, consonant), while a **Natural Minor scale** follows `W-H-W-W-H-W-W` (melancholic, introspective).\n- **Triads & Harmony**: Stacking the 1st, 3rd, and 5th scale degrees forms a triad (`C-E-G` = C Major; `A-C-E` = A minor). The **Circle of Fifths** arranges all 12 keys by ascending perfect fifths (`C -> G -> D -> A -> E -> B -> F#`), showing which keys share key signatures and why progressions like `I - V - vi - IV` or jazz `ii - V - I` resolve so naturally.",
    ),
    (
        "How do I cook pasta carbonara or sear a great steak?",
        &[
            "Give me a recipe for authentic Roman pasta carbonara.",
            "How do you cook a restaurant-quality steak?",
            "Explain the food science of cooking steak or pasta.",
        ],
        "Here are two classic culinary techniques grounded in food science:\n\n1. **Authentic Roman Pasta Carbonara (No Cream Needed)**:\n   - Crisp cubed *guanciale* (or pancetta) in a skillet until golden, then remove from direct heat.\n   - Whisk Pecorino Romano cheese, egg yolks, and toasted black pepper into a thick paste.\n   - Toss *al dente* spaghetti directly into the warm skillet with a splash of starchy pasta water and the egg-cheese mixture **off the burner** (`~62°C / 145°F`): the starch and rendered fat emulsify the egg yolks into a glossy, velvety sauce without scrambling.\n2. **Searing a Steak (The Reverse Sear)**:\n   - Pat the surface completely dry with paper towels (surface moisture boils at `100°C`, blocking the **Maillard browning reaction** which peaks at `140°C–165°C`).\n   - Warm slowly in a low oven (`120°C / 250°F`) until `5°C` below target internal temperature (`54°C / 130°F` for medium-rare), then sear 45 seconds per side in a smoking-hot cast-iron pan.",
    ),
    (
        "What are the most effective ways to study and learn new skills fast?",
        &[
            "How can I learn faster and remember what I read?",
            "Explain spaced repetition, active recall, and the Feynman technique.",
            "What are the best evidence-based study habits?",
        ],
        "Cognitive psychology consistently shows that passive re-reading and highlighting create an *illusion of competence*. Three evidence-based methods dramatically accelerate retention and mastery:\n1. **Active Recall (Retrieval Practice)**: Close the book and force your brain to reconstruct the concept or write the code from scratch before checking notes. The effortful retrieval signal triggers synaptic consolidation.\n2. **Spaced Repetition (Ebbinghaus Forgetting Curve)**: Review material just as you are about to forget it across expanding intervals (`1 day -> 3 days -> 7 days -> 21 days -> 60 days`).\n3. **The Feynman Technique**: Explain the concept in plain, jargon-free language as if teaching an intelligent twelve-year-old. Every spot where your explanation gets vague immediately pinpoints the exact gap in your mental model.",
    ),
    (
        "How do you systematically debug a difficult software bug?",
        &[
            "What is the best strategy for debugging complex code?",
            "How do I find the root cause of a weird bug?",
            "Give me a checklist for debugging software.",
        ],
        "When facing a stubborn or intermittent bug, replace guesswork with the **scientific method for debugging**:\n1. **Reproduce Deterministically First**: Create the smallest possible failing test case or script that triggers the defect 100% of the time.\n2. **Bisect the Search Space (`O(log N)`)**: Don't read 10,000 lines top-to-bottom. Check the data invariant at the midpoint of the pipeline (or use `git bisect` across commits) to cut the suspect region in half at each step.\n3. **Question Your Assumptions**: Verify that the code you edited is actually the binary being executed, that caches/service workers aren't serving stale state, and print/assert the exact concrete types and byte payloads at boundary crossings.\n4. **Fix the Root Cause + Lock It With a Regression Test**: Once your hypothesis predicts both why the bug happens and why changing line $X$ fixes it, keep the minimal reproducer as a permanent automated test.",
    ),

    // =========================================================================
    // 7. AI/ML Architecture, Transformers, RAG, Diffusion, RLHF & Hardware
    // =========================================================================
    (
        "How does Transformer self-attention (Q, K, V) work and how does UOR-R4 replace it?",
        &[
            "Explain Query, Key, and Value (Q, K, V) in Transformer attention.",
            "How does scaled dot-product attention work?",
            "Why is Transformer attention O(N^2) and how does UOR-R4 make it faster?",
        ],
        "In a standard **Transformer** (Vaswani et al., 2017), each token embedding $x_t \\in \\mathbb{R}^d$ is projected by three dense floating-point weight matrices into **Query ($Q$)**, **Key ($K$)**, and **Value ($V$)** vectors:\n$$\\text{Attention}(Q, K, V) = \\text{softmax}\\!\\left(\\frac{Q K^T}{\\sqrt{d_k}} + M\\right) V$$\nComputing $Q K^T$ across a sequence of length $N$ requires $O(N^2 d)$ floating-point multiply-accumulates and stores an $N \\times N$ attention matrix.\n\nIn **`UOR-R4` (`uor-r4.lut-stack/8`)**, we replace floating-point $Q, K, V$ projections with:\n1. **Spin-Preserving `SpiralCore` Transport**: Rotating 4D `Q12` integer lanes through 17-root Icosian quaternions (`hamilton_q12`) and 6-bit $\\text{Cl}(0,6)$ spin holonomy (`RETAIN_LAMBDA_Q31` geometric decay gives $O(N)$ linear-time recurrence).\n2. **`ServedHammingRead` Bitwise Popcount XOR Attention**: Packing rotated states into 256-bit sign sketches (`[u64; 4]`) and scanning causal history via hardware `popcount_swar_u64(q_code ^ k_code)` + `FlockSelect` (`CopyHead` and `TypeHead`).",
    ),
    (
        "What are LoRA (Low-Rank Adaptation), Quantization, and FlashAttention?",
        &[
            "Explain LoRA fine-tuning and how it works.",
            "What is FlashAttention and KV caching?",
            "How does INT8 and INT4 model quantization work?",
        ],
        "Three core efficiency techniques in modern language model engineering are:\n1. **LoRA (Low-Rank Adaptation, Hu et al. 2021)**: Freezes the pretrained weight matrix $W_0 \\in \\mathbb{R}^{d \\times k}$ and injects a trainable low-rank update $\\Delta W = B A$ where $B \\in \\mathbb{R}^{d \\times r}, A \\in \\mathbb{R}^{r \\times k}$ with rank $r \\ll \\min(d, k)$ (e.g., $r = 8$ or $16$), cutting trainable parameters by `>99%`.\n2. **FlashAttention (Dao et al.)**: Tiles $Q, K, V$ blocks into fast GPU on-chip **SRAM (shared memory)** and uses online softmax rescaling so the $N \\times N$ intermediate attention matrix is never materialized in slow HBM.\n3. **Post-Training Quantization (GPTQ / AWQ)**: Maps 16-bit floating-point weights to 8-bit or 4-bit integers with group scales—whereas `UOR-R4` trains and executes **natively in exact integer arithmetic (`Q12` / `Q24` / ternary LUTs)** from the ground up with zero quantization degradation.",
    ),
    (
        "What is Retrieval-Augmented Generation (RAG) and how do vector databases work?",
        &[
            "Explain RAG (Retrieval-Augmented Generation).",
            "How do vector embeddings and HNSW search work?",
            "When should I use RAG vs fine-tuning?",
        ],
        "**Retrieval-Augmented Generation (RAG)** grounds a language model on external documents at inference time:\n1. **Chunking & Indexing**: Documents are split into semantic passages and mapped into dense vectors (or in `UOR-R4`, 256-bit binary Hamming sketches + `Q12` coordinates).\n2. **Approximate Nearest Neighbor (ANN) Retrieval**: Given a user query, an index such as **HNSW (Hierarchical Navigable Small World graphs)**, **IVF-PQ**, or **SWAR Bitwise Popcount XOR scan** retrieves the top-$k$ most relevant passages in sub-millisecond latency.\n3. **Grounded Synthesis**: The retrieved context is prepended to the prompt so the model cites verifiable facts without hallucinating.",
    ),
    (
        "How do Diffusion Models (Stable Diffusion, Sora) generate images and video?",
        &[
            "Explain how diffusion models and flow matching work.",
            "How does Stable Diffusion turn text into an image?",
            "What is classifier-free guidance (CFG) in diffusion models?",
        ],
        "**Diffusion Models (DDPM / Latent Diffusion / Flow Matching)** learn to generate complex data by reversing a gradual noising process:\n1. **Forward Process**: Gaussian noise $\\epsilon \\sim \\mathcal{N}(0, I)$ is progressively added to an image (or a compressed VAE latent $z_0$) over $T$ timesteps until $z_T$ is pure isotropic noise.\n2. **Reverse Denoising Network (U-Net or DiT — Diffusion Transformer)**: Conditioned on a text embedding $c$ via cross-attention, the network $\\epsilon_\\theta(z_t, t, c)$ predicts the noise velocity vector to subtract at each step.\n3. **Classifier-Free Guidance (CFG)**: Combines an unconditional prediction $\\epsilon_\\theta(z_t, t, \\varnothing)$ and a text-conditioned prediction $\\epsilon_\\theta(z_t, t, c)$ with guidance weight $w > 1$ to sharpen prompt adherence.",
    ),
    (
        "Explain Reinforcement Learning from Human Feedback (RLHF), PPO, and DPO.",
        &[
            "How does RLHF align language models?",
            "What is the difference between PPO and DPO (Direct Preference Optimization)?",
            "How are AI assistants trained after pretraining?",
        ],
        "Post-pretraining alignment transforms a raw next-token predictor into a helpful assistant in three stages:\n1. **Supervised Fine-Tuning (SFT)**: Trains on curated `(prompt, high-quality demonstration)` conversations.\n2. **RLHF with PPO (Proximal Policy Optimization)**: Fits a **Reward Model** $r_\\phi(x, y)$ on human preference pairs $(y_w \\succ y_l)$ under the Bradley-Terry model, then optimizes policy $\\pi_\\theta$ to maximize expected reward minus a KL-divergence penalty $\\beta D_{\\text{KL}}(\\pi_\\theta \\parallel \\pi_{\\text{ref}})$.\n3. **DPO (Direct Preference Optimization, Rafailov et al.)**: Analytically solves for the optimal policy in closed form, optimizing $\\pi_\\theta$ directly on $(x, y_w, y_l)$ preference pairs via binary cross-entropy without training a separate reward model.",
    ),
    (
        "How does a GPU work? Explain CUDA warps, shared memory, and why RTX 5090 is fast.",
        &[
            "How do GPUs execute thousands of threads in parallel?",
            "Explain CUDA threads, blocks, warps, and shared memory.",
            "CPU vs GPU architecture.",
        ],
        "While a **CPU** optimizes for low-latency serial execution (large L1/L2/L3 caches, complex branch predictors, out-of-order superscalar pipelines), a **GPU** (such as the NVIDIA **RTX 5090** Blackwell architecture) optimizes for **high-throughput SIMT (Single Instruction, Multiple Threads)** parallelism:\n- **Warps (`32` threads)**: The fundamental unit of execution on a Streaming Multiprocessor (SM). All 32 threads in a warp execute the same instruction in lockstep.\n- **Memory Hierarchy**: Per-thread **Registers** (fastest, 1 cycle) $\\to$ on-chip **Shared Memory / L1 (`128 KB` per SM, user-managed scratchpad)** $\\to$ **L2 Cache** $\\to$ high-bandwidth **GDDR7 Global Memory (`~1.79 TB/s` on RTX 5090)**.\n- **Warp Shuffle (`__shfl_xor_sync`) & `__popcll`**: Allow `UOR-R4` kernels (`gpu/cuda_stack.cu`) to compute 256-bit Hamming XOR distances and 16-entry `FBS` table lookups entirely inside registers without global memory stalls.",
    ),

    // =========================================================================
    // 8. Systems Architecture, Databases, Networking, Security & DevOps
    // =========================================================================
    (
        "Explain database ACID properties, isolation levels, and LSM-Trees vs B-Trees.",
        &[
            "What does ACID stand for in databases?",
            "What is the difference between B-Trees and LSM-Trees (RocksDB, Cassandra)?",
            "Explain database transaction isolation levels (Read Committed, Snapshot, Serializable).",
        ],
        "Relational and transactional databases guarantee **ACID**:\n- **Atomicity**: All-or-nothing execution via a Write-Ahead Log (`WAL`).\n- **Consistency**: Every transaction preserves schema and integrity invariants.\n- **Isolation**: Concurrent transactions do not corrupt each other (**Read Committed**, **Repeatable Read / MVCC Snapshot Isolation**, and **Serializable**).\n- **Durability**: Committed changes survive power loss once flushed (`fsync`) to non-volatile storage.\n\n**Storage Engines (B-Tree vs LSM-Tree)**: **B-Trees (and B+ Trees, used in PostgreSQL and MySQL InnoDB)** update fixed-size `8 KB`/`16 KB` pages in place and excel at low-latency **read-heavy** workloads, whereas **LSM-Trees (Log-Structured Merge-Trees, used in RocksDB, Pebble, Cassandra)** buffer writes sequentially in an in-memory `MemTable` + `WAL` and flush immutable sorted **SSTables** with background compaction, excelling at **write-heavy** workloads.",
    ),
    (
        "Explain the Raft and Paxos distributed consensus algorithms.",
        &[
            "How does Raft consensus work (Leader Election and Log Replication)?",
            "What problem do Raft and Paxos solve in distributed systems?",
            "Explain split-brain and quorums in distributed consensus.",
        ],
        "Distributed consensus algorithms like **Raft** (Ongaro & Ousterhout) and **Paxos** (Lamport) allow a cluster of $2f + 1$ nodes (e.g., `3` or `5` replicas in `etcd`, CockroachDB, or Kafka KRaft) to agree on an ordered log of state-machine commands even if up to $f$ nodes crash:\n1. **Leader Election**: Time is divided into monotonically increasing **terms**. If a follower's randomized election timeout (`150–300 ms`) expires without a heartbeat, it increments the term, becomes a **Candidate**, and requests votes. A majority quorum ($\\lfloor N/2 \\rfloor + 1$) elects a single Leader per term.\n2. **Log Replication**: The Leader appends client commands to its log and sends `AppendEntries` RPCs. Once a majority of nodes persist an entry, the Leader marks it **committed** and applies it to the state machine.\n3. **Safety**: Any two majority quorums intersect in at least one node, guaranteeing that a newly elected leader always possesses all previously committed log entries.",
    ),
    (
        "How do Docker containers, Linux namespaces, and cgroups work under the hood?",
        &[
            "What is the difference between a Docker container and a Virtual Machine (VM)?",
            "How do Linux namespaces and cgroups isolate containers?",
            "Explain union filesystems (OverlayFS) in Docker.",
        ],
        "Unlike a **Virtual Machine (VM)**—which emulates hardware and boots a separate guest OS kernel on a hypervisor—a **Linux Container (Docker / containerd)** is an ordinary host OS process isolated by three kernel primitives:\n1. **Namespaces (`unshare` / `clone`)**: Partition what a process can *see*: `pid` (process IDs), `net` (network interfaces/routing tables), `mnt` (mount points), `uts` (hostname), `ipc`, and `user` (UID mapping).\n2. **Control Groups (`cgroups v2`)**: Limit and meter what a process can *consume*: CPU quota, RAM ceiling (`memory.max`), and disk/network I/O bandwidth.\n3. **OverlayFS (Copy-on-Write Union Filesystem)**: Stacks immutable content-addressed image layers beneath a thin writable container layer.",
    ),
    (
        "Explain OAuth 2.0, OpenID Connect (OIDC), JWTs, and Passkeys (WebAuthn).",
        &[
            "How does OAuth 2.0 Authorization Code Flow with PKCE work?",
            "What is the difference between Authentication (AuthN) and Authorization (AuthZ)?",
            "How do Passkeys and WebAuthn replace passwords?",
        ],
        "In modern identity and access management:\n- **OAuth 2.0 (Authorization)** with **PKCE (Proof Key for Code Exchange)**: The client generates a random `code_verifier` and sends its SHA-256 hash (`code_challenge`) to the identity provider. After user login, the client exchanges the short-lived authorization code + raw `code_verifier` for an access token, preventing authorization code interception.\n- **OpenID Connect (OIDC — Authentication)**: Layers a signed **ID Token (`JWT`: `Header.Payload.Signature`)** on top of OAuth 2.0 to verify who the user is.\n- **Passkeys (`WebAuthn` / FIDO2)**: Generates an origin-bound asymmetric keypair (`P-256` or `Ed25519`) inside the device's hardware Secure Enclave / TPM. Because the server stores only the public key and the browser binds signatures to the exact domain, passkeys are completely immune to phishing and credential stuffing.",
    ),
    (
        "Write an LRU (Least Recently Used) Cache in Rust or Python.",
        &[
            "How do you implement an O(1) LRU cache?",
            "Show me code for an LRU cache.",
            "What data structures are used to build an LRU cache?",
        ],
        "An **$O(1)$ LRU Cache** combines a **Hash Map** (for $O(1)$ key lookup) with a **Doubly Linked List** (for $O(1)$ move-to-front on access and eviction of the least-recently-used tail). In Python, `collections.OrderedDict` maintains this doubly linked hash table natively in C:\n\n```python\nfrom collections import OrderedDict\n\nclass LRUCache:\n    def __init__(self, capacity: int) -> None:\n        self.capacity = capacity\n        self.store: OrderedDict[int, int] = OrderedDict()\n\n    def get(self, key: int) -> int:\n        if key not in self.store:\n            return -1\n        self.store.move_to_end(key)\n        return self.store[key]\n\n    def put(self, key: int, value: int) -> None:\n        if key in self.store:\n            self.store.move_to_end(key)\n        self.store[key] = value\n        if len(self.store) > self.capacity:\n            self.store.popitem(last=False)\n```",
    ),

    // =========================================================================
    // 9. Engineering, Aerospace, Physics, Earth Science & Biotechnology
    // =========================================================================
    (
        "How do airplanes fly and how do rockets work in space?",
        &[
            "Explain the physics of how airplanes generate lift.",
            "How does the Tsiolkovsky rocket equation work?",
            "Bernoulli's principle vs Newton's third law in aerodynamic lift.",
        ],
        "Both aerodynamic flight and orbital rocketry follow fundamental fluid and momentum conservation laws:\n1. **How Airplanes Fly (Aerodynamic Lift)**: A wing's cambered airfoil and positive **angle of attack** accelerate airflow over the upper surface, creating a lower-pressure region above the wing (**Bernoulli's principle / Navier-Stokes pressure gradient**) while deflecting a massive volume of air downward (**Newton's Third Law**, $F = \\dot{m} \\, \\Delta v_{\\text{downwash}}$).\n2. **How Rockets Work in Vacuum (Tsiolkovsky Rocket Equation)**: Rockets do not push against surrounding air; they conserve momentum by expelling high-velocity propellant gas backward at exhaust velocity $v_e = I_{\\text{sp}} g_0$, yielding total velocity change:\n$$\\Delta v = v_e \\ln\\!\\left(\\frac{m_0}{m_f}\\right)$$\nwhere $m_0$ is initial wet mass and $m_f$ is dry mass.",
    ),
    (
        "How does GPS work and why does it require Einstein's Relativity?",
        &[
            "How does GPS calculate your location on Earth?",
            "Why would GPS fail without Special and General Relativity?",
            "Explain satellite trilateration and atomic clocks.",
        ],
        "The **Global Positioning System (GPS)** determines your 3D position $(x, y, z)$ and receiver clock bias $b$ by **trilateration** from at least **4 satellites** orbiting at `20,200 km` altitude, each carrying rubidium/cesium atomic clocks:\n1. **Time-of-Flight Ranging**: Distance $d_i = c \\, (t_{\\text{recv}} - t_{\\text{emit}, i})$. Because light travels `30 cm` (`1 foot`) per nanosecond, a `1 microsecond` clock error produces a `300 meter` position error!\n2. **Relativistic Corrections**:\n   - **Special Relativity (Kinematic Time Dilation)**: Orbiting at `3.87 km/s`, satellite clocks tick **slower** than ground clocks by **`-7.2 microseconds/day`**.\n   - **General Relativity (Gravitational Blueshift)**: Higher in Earth's gravity well, satellite clocks tick **faster** than ground clocks by **`+45.8 microseconds/day`**.\n   Without correcting the net **`+38.6 microseconds/day`** relativistic drift, GPS positions would drift by **`~11.4 kilometers every single day`**!",
    ),
    (
        "How do modern CPUs and EUV lithography print billions of nanometer transistors?",
        &[
            "How are computer chips and semiconductors manufactured?",
            "What is EUV (Extreme Ultraviolet) photolithography and ASML?",
            "How does a MOSFET / FinFET / Gate-All-Around transistor work?",
        ],
        "Modern semiconductor fabrication is one of humanity's most precise engineering feats:\n1. **The Transistor (MOSFET / FinFET / GAA Nanosheet)**: Silicon (`Si`, 4 valence electrons) is doped with Phosphorus (`n-type`, extra electrons) or Boron (`p-type`, holes). Applying a voltage to the insulated **Gate** creates an inversion channel allowing current to flow between **Source** and **Drain**.\n2. **EUV Photolithography (`13.5 nm` wavelength)**: Inside an ASML EUV scanner, a high-power $\\text{CO}_2$ laser fires twice at **`50,000` falling molten tin (`Sn`) droplets per second**, vaporizing them into a plasma that emits **`13.5 nm` Extreme Ultraviolet light**. Reflected off ultra-flat Zeiss Bragg multilayer mirrors onto a silicon wafer coated in photoresist, it patterns features just a few dozen silicon atoms wide.",
    ),
    (
        "How do lithium-ion batteries and solar panels (photovoltaics) work?",
        &[
            "How does a lithium-ion battery charge and discharge?",
            "How do solar panels turn sunlight into electricity (photovoltaic effect)?",
            "Why do lithium-ion batteries degrade over time?",
        ],
        "Clean energy relies on two complementary solid-state physics mechanisms:\n1. **Photovoltaic Solar Cells**: When a photon with energy $h\\nu \\ge E_g$ (`~1.12 eV` bandgap for silicon) strikes a **p-n junction**, it excites an electron from the valence band into the conduction band. The built-in electric field of the depletion zone separates the electron-hole pair, driving direct current (`DC`) through an external circuit.\n2. **Lithium-Ion Batteries (`Li-ion`)**: During discharge, $\\text{Li}^+$ ions de-intercalate from the **graphite anode**, travel through a liquid or solid electrolyte separator, and intercalate into the **cathode** (`NMC` or `LFP` — $\\text{LiFePO}_4$), while electrons flow through the external device. Capacity gradually fades due to growth of the **Solid Electrolyte Interphase (SEI)** layer and lithium plating during high-rate charging.",
    ),

    // =========================================================================
    // 10. Everyday Life, Career, Productivity, Literature, Philosophy & Advice
    // =========================================================================
    (
        "I'm having a stressful or overwhelming day. Can you help me reset?",
        &[
            "I'm stressed out and overwhelmed.",
            "I'm having a rough day.",
            "How do I calm down when I feel anxious or overwhelmed?",
            "Give me advice for dealing with burnout or stress.",
        ],
        "Take a slow breath—it is completely okay to pause when everything feels piled up at once. Here is a quick, practical 3-step reset you can do right now:\n\n1. **Physiological Sigh (30 seconds)**: Inhale deeply through your nose, take one short second top-up sniff at the top to fully inflate the alveoli, and exhale slowly and completely through your mouth. Doing this 2–3 times directly activates your parasympathetic nervous system (vagus nerve) and slows your heart rate.\n2. **Externalize the Mental RAM**: Write down every nagging task on a piece of paper so your working memory stops looping on them.\n3. **Pick Just One 5-Minute Action**: Ignore the rest of the list and choose the smallest, lowest-friction step you can finish in 5 minutes. Momentum cures overwhelm faster than perfection.\n\nWant to talk through what's on your plate so we can break it down together?",
    ),
    (
        "How do I overcome procrastination and stay focused on deep work?",
        &[
            "Why do I procrastinate and how can I stop?",
            "How can I focus better and get more done?",
            "Give me practical productivity tips that actually work.",
        ],
        "Psychologists have found that **procrastination is an emotional regulation problem, not a time-management problem**: we avoid tasks that trigger ambiguity, perfectionism, or fear of failure.\n\nFour techniques that reliably break the cycle:\n1. **The 2-Minute / 5-Minute Rule**: Tell yourself you only have to work on the task for 5 minutes (e.g., just open the file and write one function signature). Once you cross the activation energy barrier, the **Zeigarnik effect** (your brain's drive to finish an opened loop) takes over.\n2. **Shrink the Next Physical Action**: Replace vague goals (*\"Work on backend\"*) with concrete micro-steps (*\"Write unit test for `parse_header` in `src/lib.rs`\"*).\n3. **Timeboxed Deep Work Blocks**: Put your phone in another room for a single 45-to-60 minute uninterrupted sprint.\n4. **Separate Drafting from Editing**: Give yourself permission to write a messy first draft; it is ten times easier to refine working code or prose than an empty page.",
    ),
    (
        "Can you help me brainstorm startup or software project ideas?",
        &[
            "Give me some cool coding project ideas to build.",
            "What are some interesting startup ideas in AI or developer tools?",
            "Help me brainstorm a portfolio project.",
        ],
        "Here are four high-impact software & engineering project ideas that stand out in a portfolio or startup incubator:\n\n1. **Local-First Browser AI & Knowledge Workspace (WebAssembly + IndexedDB)**: A zero-server research and coding studio that runs deterministic integer/quantized inference locally in the browser with instant semantic search over a user's local Git repos and PDFs.\n2. **Automated Root-Cause Bisector & CI Flake Detective**: A developer tool that hooks into CI logs, clusters failure stack traces, and automatically runs parallel `git bisect` sandboxes to comment the exact offending commit and variable diff on a PR.\n3. **Deterministic Replay Debugger for Distributed Microservices**: Captures lightweight network/RPC envelopes so engineers can time-travel step-by-step through a production bug locally.\n4. **Formal Spec-to-Code Verifier Bridge**: Translates business invariants or API state machines into Lean 4 / TLA+ checks and generates property-based test suites automatically.\n\nWhich domain excites you most—developer tools, local AI, systems programming, or consumer apps?",
    ),
    (
        "How do I prepare for a software engineering technical interview?",
        &[
            "Tips for passing coding and system design interviews.",
            "How should I study for software engineering interviews?",
            "What do interviewers look for in a coding interview?",
        ],
        "A strong **software engineering interview** performance combines structured communication with algorithmic and architectural fundamentals:\n\n1. **Clarify Before You Code (First 2 Minutes)**: Repeat the problem in your own words, ask about input size bounds, edge cases (empty inputs, duplicates, negative numbers, integer overflow), and state the brute-force baseline before optimizing.\n2. **Master the 10 Core Algorithmic Patterns**: Hash Maps / Two Pointers, Sliding Window, Binary Search on Answer, BFS/DFS & Topological Sort, Heap / Priority Queue, Backtracking, Dynamic Programming (memoization $\\to$ tabulation), Monotonic Stack, Trie, and Union-Find.\n3. **Think Out Loud & Test With a Small Trace**: Walk through a 3-element example line-by-line *before* the interviewer points out an off-by-one bug, and explicitly state time and space complexity (`O(N log N)` time, `O(N)` space).\n4. **System Design Framework**: Requirements (functional + latency/QPS scale) $\\to$ API contract & data model $\\to$ high-level architecture $\\to$ deep dive into bottlenecks (sharding, caching, replication, failure modes).",
    ),
    (
        "Compare Stoicism, Existentialism, and Buddhism.",
        &[
            "What are the main ideas of Stoicism vs Existentialism?",
            "Explain the philosophy of Marcus Aurelius, Epictetus, and Seneca.",
            "How do Stoicism and Buddhism view suffering and control?",
        ],
        "Three of humanity's most practical philosophical traditions address human agency and resilience from complementary angles:\n- **Stoicism (Zeno, Seneca, Epictetus, Marcus Aurelius)**: Anchored in the **Dichotomy of Control**—distinguish strictly between what is up to you (your judgments, intentions, character, and actions) and externals outside your control (outcomes, reputation, fortune). Cultivate the four cardinal virtues: *Wisdom, Courage, Justice, and Temperance*.\n- **Buddhism (Siddhartha Gautama)**: Diagnoses *dukkha* (unsatisfactoriness/suffering) as arising from craving (*tanha*) and clinging to impermanent (*anicca*) phenomena, offering the Eightfold Path of ethical conduct, mindfulness, and wisdom.\n- **Existentialism (Kierkegaard, Nietzsche, Sartre, Camus, Beauvoir)**: Holds that *existence precedes essence*—humans have no predetermined script and must courageously author their own meaning through authentic choice and responsibility.",
    ),
    (
        "Recommend some great books across science, philosophy, sci-fi, and software engineering.",
        &[
            "What are the best books you recommend reading?",
            "Recommend some must-read books.",
            "What books should every engineer or curious person read?",
        ],
        "Here is a curated reading list across four timeless shelves:\n\n1. **Science, Math & Minds**:\n   - *Gödel, Escher, Bach: An Eternal Golden Braid* — Douglas Hofstadter\n   - *The Feynman Lectures on Physics* / *Surely You're Joking, Mr. Feynman!* — Richard Feynman\n   - *The Beginning of Infinity* — David Deutsch\n2. **Software Engineering & Systems**:\n   - *Designing Data-Intensive Applications* — Martin Kleppmann\n   - *Structure and Interpretation of Computer Programs (SICP)* — Abelson & Sussman\n   - *A Philosophy of Software Design* — John Ousterhout\n3. **Philosophy & Human Nature**:\n   - *Meditations* — Marcus Aurelius\n   - *Man's Search for Meaning* — Viktor Frankl\n   - *Thinking, Fast and Slow* — Daniel Kahneman\n4. **Science Fiction & Literature**:\n   - *Stories of Your Life and Others* / *Exhalation* — Ted Chiang\n   - *Foundation* — Isaac Asimov & *Dune* — Frank Herbert\n   - *Ficciones* — Jorge Luis Borges",
    ),
];
