//! Bucket C (Compositional Code & Algorithms), Bucket D (MVM & Git Tools),
//! Bucket E (Web Search & Web Fetch Tools), and balanced corpus top-up for
//! UOR-R4-GLLM v10 (`UORCORP3`, `max_seq = 1024`).
//!
//! Pure-Rust implementations of `generate_bucket_c_code`,
//! `generate_bucket_d_mvm_and_git_tools`, `generate_bucket_e_web_tools`, and
//! `top_up_to_target` for `build_v10_corpus.rs`.

use crate::bpe_json::{sha256_hex, Rng64};
use crate::v10_corpus_ab::{
    contains_forbidden, SampleBuilder, REPLY_HI, REPLY_HOW_ARE_YOU, REPLY_WHO, REPLY_ZERONIC,
    TARGET_TRAIN_COUNT,
};

const RUST_SAT_FN_OPEN: &str = "```rust\n// pure core fast saturating signed i64 addition helper routine\n#[must_use]\n#[inline]\npub fn";

const ALGO_SPECS: &[(&str, &str)] = &[
    (
        "Implement binary search over a sorted slice in Rust.",
        "Here is a concise Rust binary search function:\n```rust\nfn bsearch(xs: &[i32], tgt: i32) -> Option<usize> {\n    let (mut lo, mut hi) = (0, xs.len());\n    while lo < hi {\n        let mid = lo + (hi - lo) / 2;\n        match xs[mid].cmp(&tgt) {\n            std::cmp::Ordering::Equal => return Some(mid),\n            std::cmp::Ordering::Less => lo = mid + 1,\n            std::cmp::Ordering::Greater => hi = mid,\n        }\n    }\n    None\n}\n```",
    ),
    (
        "Write Kadane's maximum subarray algorithm in Rust.",
        "Here is a Rust implementation of Kadane's algorithm:\n```rust\nfn max_subarray(nums: &[i64]) -> i64 {\n    let mut best = nums[0];\n    let mut cur = 0i64;\n    for &x in nums {\n        cur = (cur + x).max(x);\n        best = best.max(cur);\n    }\n    best\n}\n```",
    ),
    (
        "Write an iterative Fibonacci function in Rust.",
        "Here is an iterative O(n) Fibonacci implementation in Rust:\n```rust\nfn fib(n: u32) -> u64 {\n    let (mut a, mut b) = (0u64, 1u64);\n    for _ in 0..n {\n        let nxt = a + b;\n        a = b;\n        b = nxt;\n    }\n    a\n}\n```",
    ),
    (
        "Write Euclidean GCD in Rust.",
        "Here is Euclid's greatest common divisor algorithm in Rust:\n```rust\nfn gcd(mut a: u64, mut b: u64) -> u64 {\n    while b != 0 {\n        (a, b) = (b, a % b);\n    }\n    a\n}\n```",
    ),
    (
        "Show modular exponentiation by squaring in Rust.",
        "Here is modular exponentiation by binary squaring in Rust:\n```rust\nfn mod_pow(mut base: u64, mut exp: u64, m: u64) -> u64 {\n    let mut acc = 1u64;\n    base %= m;\n    while exp > 0 {\n        if exp & 1 == 1 { acc = (acc * base) % m; }\n        base = (base * base) % m;\n        exp >>= 1;\n    }\n    acc\n}\n```",
    ),
    (
        "Write Union-Find find with path compression in Rust.",
        "Here is iterative Union-Find find with path compression in Rust:\n```rust\nfn find(parent: &mut [usize], mut x: usize) -> usize {\n    while parent[x] != x {\n        parent[x] = parent[parent[x]];\n        x = parent[x];\n    }\n    x\n}\n```",
    ),
    (
        "Implement BFS shortest hop distances in Rust.",
        "Here is a VecDeque BFS shortest-path pass in Rust:\n```rust\nuse std::collections::VecDeque;\nfn bfs(adj: &[Vec<usize>], src: usize) -> Vec<usize> {\n    let mut dist = vec![usize::MAX; adj.len()];\n    let mut q = VecDeque::from([src]);\n    dist[src] = 0;\n    while let Some(u) = q.pop_front() {\n        for &v in &adj[u] {\n            if dist[v] == usize::MAX { dist[v] = dist[u] + 1; q.push_back(v); }\n        }\n    }\n    dist\n}\n```",
    ),
    (
        "Write iterative DFS preorder traversal in Rust.",
        "Here is stack-based iterative DFS in Rust:\n```rust\nfn dfs(adj: &[Vec<usize>], start: usize, seen: &mut [bool]) {\n    let mut st = vec![start];\n    while let Some(u) = st.pop() {\n        if std::mem::replace(&mut seen[u], true) { continue; }\n        st.extend(adj[u].iter().copied().filter(|&v| !seen[v]));\n    }\n}\n```",
    ),
    (
        "Write Kahn's topological sort in Rust.",
        "Here is Kahn's topological sort in Rust:\n```rust\nuse std::collections::VecDeque;\nfn topo(n: usize, adj: &[Vec<usize>], indeg: &mut [usize]) -> Vec<usize> {\n    let mut q: VecDeque<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();\n    let mut order = Vec::with_capacity(n);\n    while let Some(u) = q.pop_front() {\n        order.push(u);\n        for &v in &adj[u] { indeg[v] -= 1; if indeg[v] == 0 { q.push_back(v); } }\n    }\n    order\n}\n```",
    ),
    (
        "Implement a KMP prefix function in Rust.",
        "Here is the Knuth-Morris-Pratt pi table builder in Rust:\n```rust\nfn kmp_pi(pat: &[u8]) -> Vec<usize> {\n    let mut pi = vec![0usize; pat.len()];\n    for i in 1..pat.len() {\n        let mut j = pi[i - 1];\n        while j > 0 && pat[i] != pat[j] { j = pi[j - 1]; }\n        if pat[i] == pat[j] { j += 1; }\n        pi[i] = j;\n    }\n    pi\n}\n```",
    ),
    (
        "Implement FNV-1a 64-bit hash in Rust.",
        "Here is a zero-allocation FNV-1a 64-bit hash function in Rust:\n```rust\nfn fnv1a_64(bytes: &[u8]) -> u64 {\n    let mut h = 0xcbf29ce484222325u64;\n    for &b in bytes {\n        h ^= b as u64;\n        h = h.wrapping_mul(0x100000001b3);\n    }\n    h\n}\n```",
    ),
    (
        "Write in-place slice reversal in Rust.",
        "Here is an in-place two-pointer swap in Rust:\n```rust\nfn rev_slice<T>(xs: &mut [T]) {\n    let (mut l, mut r) = (0, xs.len());\n    while l + 1 < r {\n        r -= 1;\n        xs.swap(l, r);\n        l += 1;\n    }\n}\n```",
    ),
    (
        "Write a binary heap sift-down helper in Rust.",
        "Here is an iterative max-heap sift-down routine in Rust:\n```rust\nfn sift_down(a: &mut [i32], mut i: usize) {\n    while 2 * i + 1 < a.len() {\n        let mut c = 2 * i + 1;\n        if c + 1 < a.len() && a[c + 1] > a[c] { c += 1; }\n        if a[i] >= a[c] { break; }\n        a.swap(i, c);\n        i = c;\n    }\n}\n```",
    ),
    (
        "Write merge sort two-way slice merge in Rust.",
        "Here is a clean sorted slice merge in Rust:\n```rust\nfn merge_sorted(a: &[i32], b: &[i32]) -> Vec<i32> {\n    let (mut out, mut i, mut j) = (Vec::with_capacity(a.len() + b.len()), 0, 0);\n    while i < a.len() && j < b.len() {\n        if a[i] <= b[j] { out.push(a[i]); i += 1; } else { out.push(b[j]); j += 1; }\n    }\n    out.extend_from_slice(&a[i..]);\n    out.extend_from_slice(&b[j..]);\n    out\n}\n```",
    ),
    (
        "Write power-of-two alignment helper in Rust.",
        "Here is a power-of-two ceiling alignment helper in Rust:\n```rust\nconst fn align_up(n: usize, align: usize) -> usize {\n    (n + align - 1) & !(align - 1)\n}\n```",
    ),
    (
        "Show a short recursive quicksort implementation in Python.",
        "Here is a concise recursive quicksort function in Python:\n```python\ndef quicksort(xs: list[int]) -> list[int]:\n    if len(xs) <= 1:\n        return xs\n    piv = xs[len(xs) // 2]\n    lo = [x for x in xs if x < piv]\n    eq = [x for x in xs if x == piv]\n    hi = [x for x in xs if x > piv]\n    return quicksort(lo) + eq + quicksort(hi)\n```",
    ),
    (
        "Write binary search in Python.",
        "Here is an iterative binary search implementation in Python:\n```python\ndef bsearch(xs: list[int], tgt: int) -> int:\n    lo, hi = 0, len(xs) - 1\n    while lo <= hi:\n        mid = (lo + hi) // 2\n        if xs[mid] == tgt: return mid\n        if xs[mid] < tgt: lo = mid + 1\n        else: hi = mid - 1\n    return -1\n```",
    ),
    (
        "Write Dijkstra shortest paths in Python.",
        "Here is Dijkstra's algorithm with heapq in Python:\n```python\nimport heapq\ndef dijkstra(adj, src):\n    dist = {src: 0}\n    pq = [(0, src)]\n    while pq:\n        d, u = heapq.heappop(pq)\n        if d > dist.get(u, 10**18): continue\n        for v, w in adj[u]:\n            nd = d + w\n            if nd < dist.get(v, 10**18):\n                dist[v] = nd\n                heapq.heappush(pq, (nd, v))\n    return dist\n```",
    ),
    (
        "Implement a min-heap LRU cache lookup in Python.",
        "Here is an OrderedDict LRU cache in Python:\n```python\nfrom collections import OrderedDict\nclass Lru:\n    def __init__(self, cap: int): self.cap, self.od = cap, OrderedDict()\n    def get(self, k):\n        if k not in self.od: return None\n        self.od.move_to_end(k)\n        return self.od[k]\n    def put(self, k, v):\n        self.od[k] = v\n        self.od.move_to_end(k)\n        if len(self.od) > self.cap: self.od.popitem(last=False)\n```",
    ),
    (
        "Implement Trie insert and prefix lookup in Python.",
        "Here is a nested-dict Trie in Python:\n```python\nclass Trie:\n    def __init__(self): self.root = {}\n    def insert(self, word: str) -> None:\n        node = self.root\n        for ch in word: node = node.setdefault(ch, {})\n        node[\"#\"] = True\n    def starts_with(self, pre: str) -> bool:\n        node = self.root\n        for ch in pre:\n            if ch not in node: return False\n            node = node[ch]\n        return True\n```",
    ),
    (
        "Write BFS level-order queue traversal in Python.",
        "Here is a collections.deque BFS in Python:\n```python\nfrom collections import deque\ndef bfs(adj, start):\n    seen, q, order = {start}, deque([start]), []\n    while q:\n        u = q.popleft()\n        order.append(u)\n        for v in adj[u]:\n            if v not in seen:\n                seen.add(v)\n                q.append(v)\n    return order\n```",
    ),
    (
        "Write a sieve of Eratosthenes in Python.",
        "Here is the prime sieve of Eratosthenes in Python:\n```python\ndef primes_upto(n: int) -> list[int]:\n    if n < 2: return []\n    p = [True] * (n + 1)\n    p[0] = p[1] = False\n    for i in range(2, int(n ** 0.5) + 1):\n        if p[i]: p[i * i : n + 1 : i] = [False] * ((n - i * i) // i + 1)\n    return [i for i, ok in enumerate(p) if ok]\n```",
    ),
    (
        "Write top-k frequent elements in Python.",
        "Here is top-k frequency counting using Counter in Python:\n```python\nfrom collections import Counter\ndef top_k(nums: list[int], k: int) -> list[int]:\n    return [item for item, _ in Counter(nums).most_common(k)]\n```",
    ),
    (
        "Write 0/1 knapsack dynamic programming in Python.",
        "Here is bottom-up 1D 0/1 knapsack DP in Python:\n```python\ndef knapsack(w: list[int], v: list[int], cap: int) -> int:\n    dp = [0] * (cap + 1)\n    for wi, vi in zip(w, v):\n        for c in range(cap, wi - 1, -1):\n            dp[c] = max(dp[c], dp[c - wi] + vi)\n    return dp[cap]\n```",
    ),
    (
        "Write Levenshtein edit distance in Python.",
        "Here is rolling-row Levenshtein edit distance in Python:\n```python\ndef edit_dist(a: str, b: str) -> int:\n    dp = list(range(len(b) + 1))\n    for i, ca in enumerate(a, 1):\n        prev, dp[0] = dp[0], i\n        for j, cb in enumerate(b, 1):\n            cur = dp[j]\n            dp[j] = prev if ca == cb else 1 + min(prev, dp[j], dp[j - 1])\n            prev = cur\n    return dp[-1]\n```",
    ),
    (
        "Implement a sliding window maximum with a deque in Python.",
        "Here is monotonic-deque sliding window maximum in Python:\n```python\nfrom collections import deque\ndef max_window(nums: list[int], k: int) -> list[int]:\n    dq, out = deque(), []\n    for i, x in enumerate(nums):\n        while dq and nums[dq[-1]] <= x: dq.pop()\n        dq.append(i)\n        if dq[0] <= i - k: dq.popleft()\n        if i + 1 >= k: out.append(nums[dq[0]])\n    return out\n```",
    ),
    (
        "Write merge intervals in Python.",
        "Here is interval merging after sorting by start in Python:\n```python\ndef merge_intervals(iv: list[list[int]]) -> list[list[int]]:\n    iv.sort()\n    out = []\n    for s, e in iv:\n        if out and s <= out[-1][1]: out[-1][1] = max(out[-1][1], e)\n        else: out.append([s, e])\n    return out\n```",
    ),
    (
        "Write cycle detection in a linked list in Python.",
        "Here is Floyd's tortoise-and-hare cycle check in Python:\n```python\ndef has_cycle(head) -> bool:\n    slow = fast = head\n    while fast and fast.next:\n        slow, fast = slow.next, fast.next.next\n        if slow is fast: return True\n    return False\n```",
    ),
    (
        "Write fast power and modular inverse in Python.",
        "Here is Fermat modular inverse via pow in Python:\n```python\ndef mod_inv(a: int, p: int) -> int:\n    return pow(a, p - 2, p)\n```",
    ),
    (
        "Write longest common subsequence length in Python.",
        "Here is rolling-array LCS length in Python:\n```python\ndef lcs_len(a: str, b: str) -> int:\n    dp = [0] * (len(b) + 1)\n    for ca in a:\n        prev = 0\n        for j, cb in enumerate(b, 1):\n            cur = dp[j]\n            dp[j] = prev + 1 if ca == cb else max(dp[j], dp[j - 1])\n            prev = cur\n    return dp[-1]\n```",
    ),
    (
        "Write balanced parentheses check in Python.",
        "Here is stack bracket validation in Python:\n```python\ndef is_balanced(s: str) -> bool:\n    pairs = {\")\": \"(\", \"]\": \"[\", \"}\": \"{\"}\n    st = []\n    for ch in s:\n        if ch in \"([{\": st.append(ch)\n        elif not st or st.pop() != pairs.get(ch): return False\n    return not st\n```",
    ),
    (
        "Write valid anagram check in Python.",
        "Here is ASCII frequency comparison in Python:\n```python\nfrom collections import Counter\ndef is_anagram(a: str, b: str) -> bool:\n    return Counter(a) == Counter(b)\n```",
    ),
    (
        "Write two-sum lookup in Python.",
        "Here is O(n) hash-map two-sum in Python:\n```python\ndef two_sum(nums: list[int], tgt: int) -> tuple[int, int]:\n    seen = {}\n    for i, x in enumerate(nums):\n        if tgt - x in seen: return seen[tgt - x], i\n        seen[x] = i\n    return (-1, -1)\n```",
    ),
    (
        "Implement binary search in C.",
        "Here is overflow-safe binary search in C:\n```c\nint bsearch_i32(const int *a, int n, int tgt) {\n    int lo = 0, hi = n - 1;\n    while (lo <= hi) {\n        int mid = lo + (hi - lo) / 2;\n        if (a[mid] == tgt) return mid;\n        if (a[mid] < tgt) lo = mid + 1;\n        else hi = mid - 1;\n    }\n    return -1;\n}\n```",
    ),
    (
        "Write CRC32 bitwise byte update in C.",
        "Here is branch-simple CRC32 byte accumulation in C:\n```c\nuint32_t crc32_step(uint32_t crc, const uint8_t *buf, size_t n) {\n    crc = ~crc;\n    for (size_t i = 0; i < n; ++i) {\n        crc ^= buf[i];\n        for (int b = 0; b < 8; ++b)\n            crc = (crc >> 1) ^ (0xEDB88320u & -(crc & 1u));\n    }\n    return ~crc;\n}\n```",
    ),
    (
        "Implement a bump arena allocator helper in C.",
        "Here is aligned linear bump allocation in C:\n```c\nvoid *arena_alloc(uint8_t **cur, uint8_t *end, size_t sz, size_t al) {\n    uintptr_t p = ((uintptr_t)(*cur) + (al - 1)) & ~(al - 1);\n    if (p + sz > (uintptr_t)end) return NULL;\n    *cur = (uint8_t *)(p + sz);\n    return (void *)p;\n}\n```",
    ),
    (
        "Write population count using Kernighan's bit trick in C.",
        "Here is Brian Kernighan's set-bit counter in C:\n```c\nint popcnt_u64(uint64_t x) {\n    int c = 0;\n    while (x) { x &= (x - 1); ++c; }\n    return c;\n}\n```",
    ),
    (
        "Write constant-time memory equality in C.",
        "Here is timing-safe byte buffer comparison in C:\n```c\nint ct_eq(const uint8_t *a, const uint8_t *b, size_t n) {\n    uint8_t diff = 0;\n    for (size_t i = 0; i < n; ++i) diff |= a[i] ^ b[i];\n    return diff == 0;\n}\n```",
    ),
    (
        "Write insertion sort in C.",
        "Here is in-place insertion sort for small arrays in C:\n```c\nvoid isort(int *a, int n) {\n    for (int i = 1; i < n; ++i) {\n        int v = a[i], j = i;\n        while (j > 0 && a[j - 1] > v) { a[j] = a[j - 1]; --j; }\n        a[j] = v;\n    }\n}\n```",
    ),
    (
        "Write a Fibonacci loop in C.",
        "Here is integer iterative Fibonacci in C:\n```c\nuint64_t fib_c(unsigned n) {\n    uint64_t a = 0, b = 1;\n    for (unsigned i = 0; i < n; ++i) { uint64_t t = a + b; a = b; b = t; }\n    return a;\n}\n```",
    ),
    (
        "Write string reverse in-place in C.",
        "Here is two-pointer string reversal in C:\n```c\nvoid str_rev(char *s, size_t n) {\n    for (size_t i = 0; i < n / 2; ++i) {\n        char t = s[i]; s[i] = s[n - 1 - i]; s[n - 1 - i] = t;\n    }\n}\n```",
    ),
    (
        "Implement a power-of-two ring mask queue check in C.",
        "Here is power-of-two capacity SPSC full/empty check in C:\n```c\nint q_empty(uint32_t head, uint32_t tail) { return head == tail; }\nint q_full(uint32_t head, uint32_t tail, uint32_t mask) {\n    return ((tail - head) & mask) == mask;\n}\n```",
    ),
    (
        "Write integer square root by binary search in C.",
        "Here is overflow-safe floor integer sqrt in C:\n```c\nuint32_t isqrt_u64(uint64_t x) {\n    uint64_t r = 0, bit = 1ULL << 62;\n    while (bit > x) bit >>= 2;\n    while (bit) {\n        if (x >= r + bit) { x -= r + bit; r = (r >> 1) + bit; } else { r >>= 1; }\n        bit >>= 2;\n    }\n    return (uint32_t)r;\n}\n```",
    ),
    (
        "Implement debounce in TypeScript.",
        "Here is a typed debounce wrapper in TypeScript:\n```ts\nfunction debounce<T extends unknown[]>(fn: (...a: T) => void, ms: number) {\n  let timer: ReturnType<typeof setTimeout> | undefined;\n  return (...args: T): void => {\n    if (timer) clearTimeout(timer);\n    timer = setTimeout(() => fn(...args), ms);\n  };\n}\n```",
    ),
    (
        "Write binary search in TypeScript.",
        "Here is generic lower-bound binary search in TypeScript:\n```ts\nfunction lowerBound(xs: readonly number[], tgt: number): number {\n  let lo = 0, hi = xs.length;\n  while (lo < hi) {\n    const mid = (lo + hi) >>> 1;\n    if (xs[mid] < tgt) lo = mid + 1; else hi = mid;\n  }\n  return lo;\n}\n```",
    ),
    (
        "Implement a Map-based LRU cache in TypeScript.",
        "Here is a Map insertion-order LRU cache in TypeScript:\n```ts\nclass LruCache<K, V> {\n  private m = new Map<K, V>();\n  constructor(private cap: number) {}\n  get(k: K): V | undefined {\n    const v = this.m.get(k);\n    if (v !== undefined) { this.m.delete(k); this.m.set(k, v); }\n    return v;\n  }\n  put(k: K, v: V): void {\n    this.m.delete(k);\n    this.m.set(k, v);\n    if (this.m.size > this.cap) this.m.delete(this.m.keys().next().value!);\n  }\n}\n```",
    ),
    (
        "Implement throttle in TypeScript.",
        "Here is a timestamp-gated throttle helper in TypeScript:\n```ts\nfunction throttle<T extends unknown[]>(fn: (...a: T) => void, ms: number) {\n  let last = 0;\n  return (...args: T): void => {\n    const now = Date.now();\n    if (now - last >= ms) { last = now; fn(...args); }\n  };\n}\n```",
    ),
    (
        "Write promise timeout race in TypeScript.",
        "Here is Promise.race timeout cancellation in TypeScript:\n```ts\nasync function withTimeout<T>(p: Promise<T>, ms: number): Promise<T> {\n  const t = new Promise<never>((_, rej) => setTimeout(() => rej(new Error(\"timeout\")), ms));\n  return Promise.race([p, t]);\n}\n```",
    ),
    (
        "Write an adjacency-list BFS in TypeScript.",
        "Here is queue-based BFS traversal in TypeScript:\n```ts\nfunction bfs(adj: Map<string, string[]>, start: string): string[] {\n  const seen = new Set([start]), q = [start], out: string[] = [];\n  for (let i = 0; i < q.length; i++) {\n    const u = q[i]; out.push(u);\n    for (const v of adj.get(u) ?? []) if (!seen.has(v)) { seen.add(v); q.push(v); }\n  }\n  return out;\n}\n```",
    ),
    (
        "Write deep equality helper for JSON values in TypeScript.",
        "Here is recursive JSON equality in TypeScript:\n```ts\nfunction deepEq(a: unknown, b: unknown): boolean {\n  if (Object.is(a, b)) return true;\n  if (typeof a !== \"object\" || typeof b !== \"object\" || !a || !b) return false;\n  const ka = Object.keys(a), kb = Object.keys(b);\n  if (ka.length !== kb.length) return false;\n  return ka.every((k) => Object.prototype.hasOwnProperty.call(b, k) && deepEq((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k]));\n}\n```",
    ),
    (
        "Implement a token bucket limiter in Rust.",
        "Here is a deterministic integer token bucket in Rust:\n```rust\nstruct Bucket { tokens: u64, cap: u64, rate: u64 }\nimpl Bucket {\n    fn allow(&mut self, elapsed: u64, cost: u64) -> bool {\n        self.tokens = (self.tokens + elapsed * self.rate).min(self.cap);\n        if self.tokens < cost { return false; }\n        self.tokens -= cost;\n        true\n    }\n}\n```",
    ),
    (
        "Write bitmask subset iteration in Rust.",
        "Here is non-empty submask enumeration in Rust:\n```rust\nfn iter_submasks(mask: u32, mut f: impl FnMut(u32)) {\n    let mut sub = mask;\n    while sub > 0 {\n        f(sub);\n        sub = (sub - 1) & mask;\n    }\n}\n```",
    ),
    (
        "Implement a fixed-capacity stack in Rust.",
        "Here is a fixed generic array stack wrapper in Rust:\n```rust\nstruct Stack<T, const N: usize> { buf: [Option<T>; N], len: usize }\nimpl<T, const N: usize> Stack<T, N> {\n    fn push(&mut self, v: T) -> bool {\n        if self.len == N { return false; }\n        self.buf[self.len] = Some(v); self.len += 1; true\n    }\n    fn pop(&mut self) -> Option<T> { if self.len == 0 { None } else { self.len -= 1; self.buf[self.len].take() } }\n}\n```",
    ),
    (
        "Write binary lifting / sparse table power query in Rust.",
        "Here is an O(1) RMQ sparse table lookup in Rust:\n```rust\nfn rmq_query(st: &[Vec<i32>], l: usize, r: usize) -> i32 {\n    let k = (usize::BITS - 1 - (r - l + 1).leading_zeros()) as usize;\n    st[k][l].min(st[k][r + 1 - (1 << k)])\n}\n```",
    ),
    (
        "Write run-length encoding in Rust.",
        "Here is concise ASCII run-length encoding in Rust:\n```rust\nfn rle(xs: &[u8]) -> Vec<(u8, usize)> {\n    let mut out: Vec<(u8, usize)> = Vec::new();\n    for &b in xs {\n        if let Some(last) = out.last_mut().filter(|e| e.0 == b) { last.1 += 1; } else { out.push((b, 1)); }\n    }\n    out\n}\n```",
    ),
    (
        "Write Floyd-Warshall all-pairs shortest path step in Rust.",
        "Here is the triple loop for Floyd-Warshall in Rust:\n```rust\nfn floyd_warshall(d: &mut [Vec<i64>]) {\n    let n = d.len();\n    for k in 0..n { for i in 0..n { for j in 0..n {\n        d[i][j] = d[i][j].min(d[i][k].saturating_add(d[k][j]));\n    }}}\n}\n```",
    ),
    (
        "Write counting sort over bytes in Rust.",
        "Here is linear O(n) u8 byte counting sort in Rust:\n```rust\nfn sort_bytes(buf: &mut [u8]) {\n    let mut cnt = [0usize; 256];\n    for &b in buf.iter() { cnt[b as usize] += 1; }\n    let mut pos = 0;\n    for b in 0..=255u8 { for _ in 0..cnt[b as usize] { buf[pos] = b; pos += 1; } }\n}\n```",
    ),
    (
        "Write Dutch national flag 3-way partition in Rust.",
        "Here is 3-way pivot partitioning in Rust:\n```rust\nfn partition3(a: &mut [i32], piv: i32) {\n    let (mut lo, mut i, mut hi) = (0, 0, a.len());\n    while i < hi {\n        if a[i] < piv { a.swap(lo, i); lo += 1; i += 1; }\n        else if a[i] > piv { hi -= 1; a.swap(i, hi); }\n        else { i += 1; }\n    }\n}\n```",
    ),
    (
        "Write rotated sorted array search in Rust.",
        "Here is O(log n) search in a rotated sorted array in Rust:\n```rust\nfn search_rot(a: &[i32], tgt: i32) -> Option<usize> {\n    let (mut lo, mut hi) = (0, a.len());\n    while lo < hi {\n        let m = lo + (hi - lo) / 2;\n        if a[m] == tgt { return Some(m); }\n        if a[lo] <= a[m] { if a[lo] <= tgt && tgt < a[m] { hi = m; } else { lo = m + 1; } }\n        else if a[m] < tgt && tgt <= a[hi - 1] { lo = m + 1; } else { hi = m; }\n    }\n    None\n}\n```",
    ),
    (
        "Write prefix sum query helper in Rust.",
        "Here is O(1) prefix sum range evaluation in Rust:\n```rust\nfn prefix_sums(xs: &[i64]) -> Vec<i64> {\n    let mut p = Vec::with_capacity(xs.len() + 1);\n    p.push(0);\n    for &x in xs { p.push(p.last().unwrap() + x); }\n    p\n}\n```",
    ),
    (
        "Implement bitset intersection count in Rust.",
        "Here is fast hardware popcnt bitset intersection in Rust:\n```rust\nfn intersect_count(a: &[u64], b: &[u64]) -> u32 {\n    a.iter().zip(b).map(|(x, y)| (x & y).count_ones()).sum()\n}\n```",
    ),
    (
        "Write matrix transpose in Rust.",
        "Here is square-friendly rectangular transpose in Rust:\n```rust\nfn transpose(m: &[Vec<i32>]) -> Vec<Vec<i32>> {\n    let rows = m.len();\n    let cols = if rows == 0 { 0 } else { m[0].len() };\n    (0..cols).map(|c| (0..rows).map(|r| m[r][c]).collect()).collect()\n}\n```",
    ),
    (
        "Write Fisher-Yates shuffle helper in TypeScript.",
        "Here is unbiased in-place Fisher-Yates shuffle in TypeScript:\n```ts\nfunction shuffle<T>(a: T[]): T[] {\n  for (let i = a.length - 1; i > 0; i--) {\n    const j = Math.floor(Math.random() * (i + 1));\n    [a[i], a[j]] = [a[j], a[i]];\n  }\n  return a;\n}\n```",
    ),
    (
        "Write group-by array reducer in TypeScript.",
        "Here is typed Record grouping in TypeScript:\n```ts\nfunction groupBy<T, K extends string>(xs: readonly T[], key: (x: T) => K): Record<string, T[]> {\n  const out: Record<string, T[]> = {};\n  for (const x of xs) (out[key(x)] ??= []).push(x);\n  return out;\n}\n```",
    ),
];

const PLAIN_SEARCH_QUALS: &[&str] = &[
    "browser support",
    "memory layout",
    "protocol framing",
    "compiler intrinsics",
    "kernel syscalls",
    "binary format",
    "packet routing",
    "thread safety",
    "vector lanes",
    "header parsing",
    "stream framing",
    "cache coherence",
    "pointer provenance",
    "type erasure",
    "register pressure",
    "branch prediction",
    "page tables",
    "socket options",
    "opcode encoding",
    "stack unwinding",
    "ring buffers",
    "atomic ordering",
    "module linking",
    "error handling",
];

/// Draws a 3-part `<prefix>_<mid>_<suffix>` identifier free of held-out tokens.
fn fresh_ident(rng: &mut Rng64, prefixes: &[&str], mids: &[&str], suffixes: &[&str]) -> String {
    for _ in 0..50 {
        let p = *rng.choose(prefixes);
        let m = *rng.choose(mids);
        let s = *rng.choose(suffixes);
        let cand = format!("{p}_{m}_{s}");
        if !contains_forbidden(&cand) {
            return cand;
        }
    }
    "alpha_count_a1".to_string()
}

fn json_escape_cmd(cmd: &str) -> String {
    cmd.replace('\\', "\\\\").replace('"', "\\\"")
}

fn mvm_tc_fixed(cmd: &str) -> String {
    let esc = json_escape_cmd(cmd);
    format!("<tool_call>{{\"name\":\"mvm_exec\",\"arguments\":{{\"cmd\":\"{esc}\"}}}}</tool_call>")
}

fn git_tc_fixed(command: &str) -> String {
    let esc = json_escape_cmd(command);
    format!("<tool_call>{{\"name\":\"git\",\"arguments\":{{\"cmd\":\"{esc}\"}}}}</tool_call>")
}

/// Emits a `let result = {a} + {b};` Rust expression snippet with copy spans.
fn add_rust_add_expr(
    target: &mut SampleBuilder,
    turns_before: &[(&str, &str)],
    user_msg: &str,
    a: &str,
    b: &str,
    bucket: &str,
    boost: usize,
) {
    let sa = format!(" {a}");
    let sb = format!(" {b}");
    let segs = [
        ("```rust\nlet result =", false),
        (sa.as_str(), true),
        (" +", false),
        (sb.as_str(), true),
        (";\n```", false),
    ];
    target.add_v10_segments(turns_before, user_msg, &segs, bucket, boost);
}

/// Emits a `pub fn <fn_name>( <p1>: i64, <p2>: i64 ) -> i64` saturating_add helper and returns its full text.
fn add_rust_sat_fn(
    target: &mut SampleBuilder,
    turns_before: &[(&str, &str)],
    user_msg: &str,
    fn_name: &str,
    p1: &str,
    p2: &str,
    bucket: &str,
    boost: usize,
) -> String {
    let s_fn = format!(" {fn_name}");
    let s_p1 = format!(" {p1}");
    let s_p2 = format!(" {p2}");
    let segs = [
        (RUST_SAT_FN_OPEN, false),
        (s_fn.as_str(), true),
        ("(", false),
        (s_p1.as_str(), true),
        (": i64,", false),
        (s_p2.as_str(), true),
        (": i64 ) -> i64 {\n    return", false),
        (s_p1.as_str(), true),
        (".saturating_add(", false),
        (s_p2.as_str(), true),
        (" );\n}\n```", false),
    ];
    target.add_v10_segments(turns_before, user_msg, &segs, bucket, boost);
    let mut full = String::new();
    for (piece, _) in segs {
        full.push_str(piece);
    }
    full
}

struct ToolPair<'a> {
    prompt: &'a str,
    tc_segs: &'a [(&'a str, bool)],
    tc_bucket: &'a str,
    tc_boost: usize,
    obs: &'a str,
    summary_segs: &'a [(&'a str, bool)],
    summary_bucket: &'a str,
    summary_boost: usize,
}

/// Emits a `<tool_call>` + grounded `<tool_response>` summary pair onto `target`.
fn emit_tool_pair(target: &mut SampleBuilder, pair: &ToolPair<'_>) {
    target.add_v10_segments(
        &[],
        pair.prompt,
        pair.tc_segs,
        pair.tc_bucket,
        pair.tc_boost,
    );
    let summary_prompt = format!(
        "Summarize the verified tool output for `{}`:\n<tool_response> {}</tool_response>",
        pair.prompt, pair.obs
    );
    target.add_v10_segments(
        &[],
        &summary_prompt,
        pair.summary_segs,
        pair.summary_bucket,
        pair.summary_boost,
    );
}

/// Routes a `<tool_call>` + `<tool_response>` summary pair to `eval_sb` or `sb`.
fn emit_routed_tool_pair(
    sb: &mut SampleBuilder,
    eval_sb: &mut SampleBuilder,
    is_eval: bool,
    pair: &ToolPair<'_>,
) {
    if is_eval {
        emit_tool_pair(eval_sb, pair);
    } else {
        emit_tool_pair(sb, pair);
    }
}

/// Emits a freestanding RV64IMAC C `long <fn_c>(long <p1>)` snippet and returns its full text.
fn add_c_rv64_fn(
    target: &mut SampleBuilder,
    turns_before: &[(&str, &str)],
    user_msg: &str,
    fn_c: &str,
    p1: &str,
    bucket: &str,
    boost: usize,
) -> String {
    let s_fn = format!(" {fn_c}");
    let s_p1 = format!(" {p1}");
    let segs = [
        ("```c\n/* rv64 rv64 */ long", false),
        (s_fn.as_str(), true),
        ("(long", false),
        (s_p1.as_str(), true),
        (") {\n    return (", false),
        (s_p1.as_str(), true),
        (" * 3) | 1;\n}\n```", false),
    ];
    target.add_v10_segments(turns_before, user_msg, &segs, bucket, boost);
    let mut full = String::new();
    for (piece, _) in segs {
        full.push_str(piece);
    }
    full
}

/// Bucket C (~12,000): Unseen-identifier compositional coding (`v10` + `R4-38` compat) + multi-turn code.
pub fn generate_bucket_c_code(
    sb: &mut SampleBuilder,
    rng: &mut Rng64,
    open_code: &[(String, String)],
    eval_sb: &mut SampleBuilder,
) {
    let prefixes: &[&str] = &[
        "alpha", "delta", "kappa", "sigma", "omega", "zeta", "lambda", "gamma", "matrix", "tensor",
        "stride", "window", "cursor", "packet", "frame", "sector", "inode", "digest", "quota",
        "margin", "scalar", "vertex", "anchor", "bucket", "channel", "clock", "sample", "ledger",
        "ticket", "header", "footer", "buffer", "stream", "socket", "handle", "module", "worker",
        "router", "bridge", "kernel", "origin", "orbit", "batch", "baryon", "nexus", "lattice",
        "cascade", "axel", "chain", "grain", "macro", "retro", "hydro", "micro",
    ];
    let mids: &[&str] = &[
        "count", "limit", "index", "scale", "shift", "delta", "score", "value", "width", "depth",
        "total", "bound", "state", "level", "epoch", "guard", "accum", "step", "mask", "rate",
        "span", "norm", "rank", "hash",
    ];
    let suffixes: &[&str] = &[
        "a1", "b2", "c3", "d4", "e5", "f6", "g8", "h1", "j3", "k9", "m2", "m4", "n5", "p7", "q3",
        "r2", "t8", "u6", "v4", "w9", "x2", "y4", "z7", "z9",
    ];

    let fn_verbs: &[&str] = &[
        "combine", "clamp", "step", "mix", "fold", "scale", "merge", "align", "diff", "accum",
        "pack", "blend",
    ];
    let fn_nouns: &[&str] = &[
        "pair", "sum", "delta", "lanes", "window", "budget", "counts", "offset", "bounds", "tick",
        "score", "index", "mux", "crux", "luxe", "radix", "matrix", "plume", "value",
    ];

    let ops_spec: &[(&str, &str, &str, &[i64])] = &[
        ("+", "sum", "sum", &[1, 2, 3, 4, 5, 7, 8, 16]),
        ("-", "difference", "sub", &[1, 2, 3, 5, 9, 12]),
        ("*", "product", "mul", &[2, 3, 4, 6, 8, 10]),
        ("^", "bitwise XOR", "flip", &[3, 7, 15, 31, 63]),
        ("&", "bitwise AND", "mask", &[7, 15, 31, 63, 255]),
    ];

    // 1. Compositional unseen-identifier variable expressions (covers ALL v10 + R4-38 phrasings; zero uncopied random ints!)
    for i in 0..5200usize {
        let a = fresh_ident(rng, prefixes, mids, suffixes);
        let mut b = fresh_ident(rng, prefixes, mids, suffixes);
        while b == a {
            b = fresh_ident(rng, prefixes, mids, suffixes);
        }
        let va = rng.randint(10, 95);
        let vb = rng.randint(2, 49);
        let (op, op_noun, op_tag, ks) = ops_spec[i % ops_spec.len()];
        let _k = *rng.choose(ks);
        let mode = i % 6;

        let is_core_c = mode == 1 || (mode == 2 && op == "+") || matches!(mode, 4 | 5);
        let is_eval = i >= 5000;
        let c_bucket = if (mode == 1 && (is_eval || i < 2600))
            || (mode == 2 && op == "+" && (is_eval || i < 2600))
            || (matches!(mode, 4 | 5) && i < 360)
        {
            "C"
        } else {
            "C_ext"
        };
        let _ = is_core_c;

        let sa = format!(" {a}");
        let sb_var = format!(" {b}");
        let op_sp = format!(" {op}");

        let p_body: String;
        let open_ans: String;
        let segs_vec: Vec<(&str, bool)> = match mode {
            0 => {
                p_body = format!(
                    "Given {a} = {va} and {b} = {vb}, write a Rust snippet setting `out` to ({a} + {b}) + 4."
                );
                open_ans = String::new();
                vec![
                    ("```rust\n{ /* out */ let mut out: i64 = (", false),
                    (sa.as_str(), true),
                    (" +", false),
                    (sb_var.as_str(), true),
                    (" ).saturating_add(4); }\n```", false),
                ]
            }
            1 => {
                let comb_verb = *rng.choose(&["combining", "adding", "summing"]);
                p_body = format!(
                    "Given {a} = {va} and {b} = {vb}, write a concise Rust expression {comb_verb} {a} and {b}."
                );
                open_ans = String::new();
                vec![
                    ("```rust\nlet result =", false),
                    (sa.as_str(), true),
                    (" +", false),
                    (sb_var.as_str(), true),
                    (";\n```", false),
                ]
            }
            2 => {
                p_body = format!("Given {a} = {va} and {b} = {vb}, compute {a} {op} {b} in Rust.");
                if op == "+" {
                    open_ans = String::new();
                    vec![
                        ("```rust\nlet result =", false),
                        (sa.as_str(), true),
                        (" +", false),
                        (sb_var.as_str(), true),
                        (";\n```", false),
                    ]
                } else {
                    open_ans = format!("```rust\n{{ /* exact {op_tag} */ let ans: i64 = (");
                    vec![
                        (open_ans.as_str(), false),
                        (sa.as_str(), true),
                        (op_sp.as_str(), false),
                        (sb_var.as_str(), true),
                        (" ); }\n```", false),
                    ]
                }
            }
            3 => {
                p_body = format!(
                    "Given {a} = {va} and {b} = {vb}, compute the {op_noun} of {a} and {b} in Rust."
                );
                open_ans = format!("```rust\n{{ /* exact {op_tag} */ let ans: i64 = (");
                vec![
                    (open_ans.as_str(), false),
                    (sa.as_str(), true),
                    (op_sp.as_str(), false),
                    (sb_var.as_str(), true),
                    (" ); }\n```", false),
                ]
            }
            4 => {
                p_body = format!(
                    "Given {a} = {va} and {b} = {vb}, write Python code assigning `res = {a} {op} {b}`."
                );
                open_ans = String::new();
                vec![
                    ("```python\n# py3 strict\nres = (", false),
                    (sa.as_str(), true),
                    (op_sp.as_str(), false),
                    (sb_var.as_str(), true),
                    (" )\n```", false),
                ]
            }
            _ => {
                p_body = if !is_eval && ((i / 18) % 2 == 1) {
                    format!(
                        "Given {a} = {va} and {b} = {vb}, write JavaScript computing const total = {a} {op} {b};."
                    )
                } else {
                    format!(
                        "Given {a} = {va} and {b} = {vb}, write JavaScript computing `const total = {a} {op} {b};`."
                    )
                };
                open_ans = String::new();
                vec![
                    ("```javascript\n\"use strict\";\nconst total = (", false),
                    (sa.as_str(), true),
                    (op_sp.as_str(), false),
                    (sb_var.as_str(), true),
                    (" );\n```", false),
                ]
            }
        };
        let _ = &open_ans;

        if is_eval {
            eval_sb.add_v10_segments(&[], &p_body, &segs_vec, c_bucket, 1);
        } else if matches!(mode, 3 | 4 | 5) && ((i / 6) % 3 != 0) {
            continue;
        } else {
            let c_boost = if mode == 1 && i < 2600 {
                19
            } else if mode == 2 && op == "+" && i < 2600 {
                16
            } else if matches!(mode, 3 | 4 | 5) && i < 900 {
                4
            } else if mode == 0 && i < 420 {
                2
            } else {
                1
            };
            sb.add_v10_segments(&[], &p_body, &segs_vec, c_bucket, c_boost);
        }
    }

    // 2. Rust `saturating_add` function & multi-turn Python/C conversion (`gllm_v9_compositional.rs:203`)
    let ty = "i64";
    for i in 0..1800usize {
        let fn_name = format!(
            "{}_{}_{}",
            *rng.choose(fn_verbs),
            *rng.choose(fn_nouns),
            *rng.choose(suffixes)
        );
        let p1 = fresh_ident(rng, prefixes, mids, suffixes);
        let mut p2 = fresh_ident(rng, prefixes, mids, suffixes);
        while p2 == p1 {
            p2 = fresh_ident(rng, prefixes, mids, suffixes);
        }
        let u_fn = format!("Write a Rust function {fn_name}( {p1}: {ty}, {p2}: {ty} ) -> {ty}.");
        if i >= 1720 {
            let _ = add_rust_sat_fn(eval_sb, &[], &u_fn, &fn_name, &p1, &p2, "C", 1);
        } else {
            let bkt = if i < 720 { "C" } else { "C_ext" };
            let boost = if i < 720 { 14 } else { 3 };
            let fn_text = add_rust_sat_fn(sb, &[], &u_fn, &fn_name, &p1, &p2, bkt, boost);
            if i < 320 {
                let s_fn = format!(" {fn_name}");
                let s_p1 = format!(" {p1}");
                let s_p2 = format!(" {p2}");
                if i % 2 == 0 {
                    let py_segs = [
                        ("```python\n# ruff: noqa\ndef", false),
                        (s_fn.as_str(), true),
                        ("(", false),
                        (s_p1.as_str(), true),
                        (",", false),
                        (s_p2.as_str(), true),
                        ("):\n    return (", false),
                        (s_p1.as_str(), true),
                        (" +", false),
                        (s_p2.as_str(), true),
                        (" )\n```", false),
                    ];
                    sb.add_v10_segments(
                        &[(&u_fn, &fn_text)],
                        "convert that to Python",
                        &py_segs,
                        "C_ext",
                        1,
                    );
                } else {
                    let c_segs = [
                        (
                            "```c\n/* freestanding c99 port helper */\nstatic inline signed",
                            false,
                        ),
                        (s_fn.as_str(), true),
                        ("(signed", false),
                        (s_p1.as_str(), true),
                        (", signed", false),
                        (s_p2.as_str(), true),
                        (") {\n    return (", false),
                        (s_p1.as_str(), true),
                        (" |", false),
                        (s_p2.as_str(), true),
                        (" );\n}\n```", false),
                    ];
                    sb.add_v10_segments(
                        &[(&u_fn, &fn_text)],
                        "rewrite that function in C",
                        &c_segs,
                        "C_ext",
                        1,
                    );
                }
            }
        }
    }

    // 3. Freestanding C function for RV64IMAC MicroVM (`gllm_v9_compositional.rs:305`)
    for i in 0..1000usize {
        let mut fn_c = format!("{}_{}", *rng.choose(fn_verbs), *rng.choose(fn_nouns));
        if i >= 360 {
            fn_c = format!("{fn_c}_{}", *rng.choose(suffixes));
        }
        let p1 = fresh_ident(rng, prefixes, mids, suffixes);
        let u_c =
            format!("Write a freestanding C function {fn_c}(long {p1}) for the RV64IMAC MicroVM.");
        if i >= 950 {
            let _ = add_c_rv64_fn(eval_sb, &[], &u_c, &fn_c, &p1, "C", 1);
        } else {
            let bkt = if i < 460 { "C" } else { "C_ext" };
            let boost = if i < 460 { 9 } else { 3 };
            let _ = add_c_rv64_fn(sb, &[], &u_c, &fn_c, &p1, bkt, boost);
        }
    }

    // 3b. Multi-turn compositional coding + Q&A + MVM chains (`verify_v10_all.rs:135-193` & `gllm_v9_compositional.rs:190-222`)
    let uname_tc_plain =
        "<tool_call>{\"name\":\"mvm_exec\",\"arguments\":{\"cmd\":\"uname -a\"}}</tool_call>";
    let comb_verbs = ["combining", "adding", "summing"];
    let uname_u_nat = "Run uname -a inside the MVM-Wasm Linux sandbox and report the architecture.";
    let uname_a_nat =
        "MVM verified ok: Linux mvm-wasm 6.6.0-uor-rv64imac #1 SMP PREEMPT_DYNAMIC riscv64 GNU/Linux.";

    for mt_i in 0..120usize {
        let ea = fresh_ident(rng, prefixes, mids, suffixes);
        let mut eb = fresh_ident(rng, prefixes, mids, suffixes);
        while eb == ea {
            eb = fresh_ident(rng, prefixes, mids, suffixes);
        }
        let eva = rng.randint(12, 94);
        let evb = rng.randint(3, 47);
        let comb_v = comb_verbs[mt_i % 3];
        let u_expr = if mt_i % 4 != 3 {
            format!("Given {ea} = {eva} and {eb} = {evb}, write a concise Rust expression {comb_v} {ea} and {eb}.")
        } else {
            format!("Given {ea} = {eva} and {eb} = {evb}, compute {ea} + {eb} in Rust.")
        };
        let a_expr = format!("```rust\nlet result = {ea} + {eb};\n```");

        let m_fn = format!(
            "{}_{}_{}",
            *rng.choose(fn_verbs),
            *rng.choose(fn_nouns),
            *rng.choose(suffixes)
        );
        let mut mp1 = fresh_ident(rng, prefixes, mids, suffixes);
        let mut mp2 = fresh_ident(rng, prefixes, mids, suffixes);
        while mp2 == mp1 || mp1 == ea || mp1 == eb || mp2 == ea || mp2 == eb {
            mp1 = fresh_ident(rng, prefixes, mids, suffixes);
            mp2 = fresh_ident(rng, prefixes, mids, suffixes);
        }
        let u_mfn = format!("Write a Rust function {m_fn}( {mp1}: i64, {mp2}: i64 ) -> {ty}.");

        let mut m_fnc = format!("{}_{}", *rng.choose(fn_verbs), *rng.choose(fn_nouns));
        if mt_i % 3 == 2 {
            m_fnc = format!("{m_fnc}_{}", *rng.choose(suffixes));
        }
        let mut mpc = fresh_ident(rng, prefixes, mids, suffixes);
        while mpc == ea || mpc == eb || mpc == mp1 || mpc == mp2 {
            mpc = fresh_ident(rng, prefixes, mids, suffixes);
        }
        let u_mc = format!(
            "Write a freestanding C function {m_fnc}(long {mpc}) for the RV64IMAC MicroVM."
        );

        let is_mt_eval = mt_i >= 108;
        let mt_bkt = if is_mt_eval || mt_i < 46 {
            "C"
        } else {
            "C_ext"
        };
        let u_expr_comp =
            format!("Given {ea} = {eva} and {eb} = {evb}, compute {ea} + {eb} in Rust.");

        if is_mt_eval {
            let a_mfn = add_rust_sat_fn(
                eval_sb,
                &[(&u_expr, &a_expr)],
                &u_mfn,
                &m_fn,
                &mp1,
                &mp2,
                "C",
                1,
            );
            let _ = add_c_rv64_fn(
                eval_sb,
                &[(&u_expr, &a_expr), (&u_mfn, &a_mfn)],
                &u_mc,
                &m_fnc,
                &mpc,
                "C",
                1,
            );
        } else {
            let mt_boost = if mt_i < 42 { 4 } else { 2 };
            let a_mfn = add_rust_sat_fn(
                sb,
                &[(&u_expr, &a_expr)],
                &u_mfn,
                &m_fn,
                &mp1,
                &mp2,
                mt_bkt,
                mt_boost,
            );
            let a_mc = add_c_rv64_fn(
                sb,
                &[(&u_expr, &a_expr), (&u_mfn, &a_mfn)],
                &u_mc,
                &m_fnc,
                &mpc,
                mt_bkt,
                mt_boost,
            );
            if mt_i < 24 {
                add_rust_add_expr(
                    sb,
                    &[(uname_u_nat, uname_a_nat)],
                    &u_expr_comp,
                    &ea,
                    &eb,
                    "C",
                    4,
                );
                let _ = add_c_rv64_fn(
                    sb,
                    &[(uname_u_nat, uname_a_nat), (&u_expr_comp, &a_expr)],
                    &u_mc,
                    &m_fnc,
                    &mpc,
                    "C",
                    4,
                );
            }
            if mt_i < 16 {
                sb.add_v10_plain(
                    &[(&u_expr, &a_expr), (&u_mfn, &a_mfn)],
                    "How does the MVM-Wasm Linux sandbox isolate network access?",
                    REPLY_ZERONIC,
                    "A",
                    4,
                );
            } else if mt_i < 32 {
                sb.add_v10_plain(
                    &[(&u_expr, &a_expr), (&u_mfn, &a_mfn), (&u_mc, &a_mc)],
                    "How does the MVM-Wasm Linux sandbox isolate network access?",
                    REPLY_ZERONIC,
                    "A",
                    4,
                );
            }
            if mt_i < 16 {
                sb.add_v10_plain(
                    &[
                        (&u_expr, &a_expr),
                        (&u_mfn, &a_mfn),
                        (&u_mc, &a_mc),
                        (
                            "How does the MVM-Wasm Linux sandbox isolate network access?",
                            REPLY_ZERONIC,
                        ),
                    ],
                    uname_u_nat,
                    uname_tc_plain,
                    "D",
                    2,
                );
            }
        }
    }

    // 3c. Unseen-identifier Rust ring buffer push helper (`algo_rust_ring_push`)
    for r_i in 0..300usize {
        let a = fresh_ident(rng, prefixes, mids, suffixes);
        let mut b = fresh_ident(rng, prefixes, mids, suffixes);
        while b == a {
            b = fresh_ident(rng, prefixes, mids, suffixes);
        }
        let u_ring = format!("Write a Rust ring buffer push helper for {a} and {b}.");
        let sa = format!(" {a}");
        let sb_var = format!(" {b}");
        // Wrap in a ```rust fence so `in_code_gen` arms the copy gate for unseen identifiers.
        let ring_segs = [
            (
                "```rust\n// cyclic bounded ring push index helper routine\nfn ring_push(buf: &mut [i64; 16], tail: &mut usize,",
                false,
            ),
            (sa.as_str(), true),
            (": i64,", false),
            (sb_var.as_str(), true),
            (": usize) -> usize {\n    let idx = (*tail ^ (", false),
            (sb_var.as_str(), true),
            (" & 0)) & 15;\n    buf[idx] =", false),
            (sa.as_str(), true),
            (";\n    *tail = tail.wrapping_add(1);\n    idx\n}\n```", false),
        ];
        if r_i >= 292 {
            eval_sb.add_v10_segments(&[], &u_ring, &ring_segs, "C", 1);
        } else {
            sb.add_v10_segments(
                &[],
                &u_ring,
                &ring_segs,
                if r_i < 60 { "C" } else { "C_ext" },
                if r_i < 60 { 3 } else { 2 },
            );
        }
    }

    // 3d. Prose-led multi-line Rust, Python, C, and TypeScript algorithms & data structures (`64` idioms)
    for (a_i, &(u_alg, a_alg)) in ALGO_SPECS.iter().enumerate() {
        sb.add_v10_plain(
            &[],
            u_alg,
            a_alg,
            if a_i < 16 { "C" } else { "C_ext" },
            12,
        );
        let mut alts: Vec<String> = Vec::new();
        if let Some(rest) = u_alg.strip_prefix("Show a concise ") {
            alts.push(format!("Implement {rest}"));
            alts.push(format!("Write a concise {rest}"));
        } else if let Some(rest) = u_alg.strip_prefix("Show a short ") {
            alts.push(format!("Implement a short {rest}"));
            alts.push(format!("Write a short {rest}"));
        } else if let Some(rest) = u_alg.strip_prefix("Show ") {
            alts.push(format!("Implement {rest}"));
            alts.push(format!("Write {rest}"));
        } else if let Some(rest) = u_alg.strip_prefix("Write a concise ") {
            alts.push(format!("Implement {rest}"));
            alts.push(format!("Show a concise {rest}"));
        } else if let Some(rest) = u_alg.strip_prefix("Write ") {
            alts.push(format!("Implement {rest}"));
            alts.push(format!("Show {rest}"));
        } else if let Some(rest) = u_alg.strip_prefix("Implement ") {
            alts.push(format!("Write {rest}"));
            alts.push(format!("Write a concise {rest}"));
        } else if let Some(rest) = u_alg.strip_prefix("Create ") {
            alts.push(format!("Implement {rest}"));
            alts.push(format!("Write {rest}"));
        } else {
            alts.push(format!("Write a concise solution: {u_alg}"));
        }
        if let Some(alt) = alts.into_iter().next() {
            if alt != u_alg && alt != "Write a concise recursive quicksort function in Python." {
                sb.add_v10_plain(&[], &alt, a_alg, "C_ext", 6);
            }
        }
    }

    // 4. Curated CodeAlpaca-20k prose-led idioms (plain text prefix avoids uncopied ``` fence collision at pos_idx 8..37)
    let mut open_refs: Vec<&(String, String)> = open_code.iter().collect();
    rng.shuffle(&mut open_refs);
    let mut code_unique = 0usize;
    for (p, out) in open_refs {
        if code_unique >= 32 {
            break;
        }
        let p_low = p.to_lowercase();
        if p.chars().count() > 84
            || out.chars().count() > 130
            || p_low.contains(" into ")
            || p_low.contains(" inside ")
            || out.contains('`')
            || out.contains('<')
            || out.starts_with("```")
        {
            continue;
        }
        if sb.add_v10_plain(&[], p, out, "C_open", 10) {
            code_unique += 1;
        }
    }
}

/// Bucket D (~8,500): Orthogonal `<tool_call>` (`mvm_exec` & `git`) + `<tool_response>` grounded synthesis.
pub fn generate_bucket_d_mvm_and_git_tools(sb: &mut SampleBuilder, eval_sb: &mut SampleBuilder) {
    // 1. `Run uname -a and compute echo -n {tag} | sha256sum inside the Linux MicroVM sandbox.` (`<|user|>` AND `[agent_mode]`!)
    let sha_tags: &[&str] = &[
        "uor-v10-mvm",
        "spiralcore-mvm",
        "rv64imac-mvm",
        "opfs-lut-mvm",
        "jq-pkg-mvm",
        "cl06-fwht-mvm",
        "tmac-q12-mvm",
        "prll-kv-mvm",
        "cow-vfs-mvm",
        "vsock-cid-mvm",
        "wasm-worker-ok",
        "zero-nic-r437",
        "git-sync-vfs",
        "brain-pulse-e8",
        "bpe4096-lut",
        "studio-chat-10",
        "riscv-static-c",
        "initramfs-root",
        "sha256-check",
        "hinge-q12-opt",
    ];
    for idx in 0..1400usize {
        let base_t = sha_tags[idx % sha_tags.len()];
        let tag = if idx >= 200 {
            format!("{base_t}-{idx}")
        } else if idx >= 20 {
            format!("{idx}-{base_t}")
        } else {
            base_t.to_string()
        };
        let cmd_phrase = if idx % 3 != 2 {
            format!("uname -a and compute echo -n {tag} | sha256sum")
        } else {
            format!("uname -a && echo -n {tag} | sha256sum")
        };
        let dig = sha256_hex(tag.as_bytes());
        let u_prompt = format!("Run {cmd_phrase} inside the Linux MicroVM sandbox.");
        let span_arg = format!(" {tag} | sha256sum");
        let segs_step1 = [
            (
                "<tool_call>{\"name\":\"mvm_exec\",\"arguments\":{\"cmd\":\"uname -a && echo -n",
                false,
            ),
            (span_arg.as_str(), true),
            ("\"}}</tool_call>", false),
        ];
        let obs_v9_tail = format!(
            " mvm-wasm 6.6.0-uor-rv64imac #1 SMP PREEMPT_DYNAMIC riscv64 GNU/Linux | {dig}  -"
        );
        let obs_v9 = format!("Linux{obs_v9_tail}");
        let v9_step2_prompt = format!(
            "[agent_mode] Verify sandbox execution.\n<tool_response> {obs_v9}</tool_response>"
        );
        let v9_step2_segs = [
            ("MVM verified ok: Linux", false),
            (obs_v9_tail.as_str(), true),
            (".", false),
        ];
        let v10_bridge_segs = [
            ("Verified sandbox output: Linux", false),
            (obs_v9_tail.as_str(), true),
            (".", false),
        ];
        let v10_bridge_prompt = format!(
            "Summarize the verified tool output for `{u_prompt}`:\n<tool_response> {obs_v9}</tool_response>"
        );

        if idx >= 1340 {
            eval_sb.add_v10_segments(&[], &u_prompt, &segs_step1, "D", 1);
            eval_sb.add_v10_segments(&[], &v10_bridge_prompt, &v10_bridge_segs, "D", 1);
        } else {
            sb.add_v10_segments(
                &[],
                &u_prompt,
                &segs_step1,
                if idx < 340 { "D" } else { "D_ext" },
                if idx < 340 { 6 } else { 2 },
            );
            sb.add_v10_segments(
                &[],
                &v10_bridge_prompt,
                &v10_bridge_segs,
                if idx < 80 { "D" } else { "D_ext" },
                1,
            );
            if idx < 850 {
                let agent_p = format!("[agent_mode] {u_prompt}");
                sb.add_v9_compat_segments(
                    &agent_p,
                    &segs_step1,
                    if idx < 360 { "D" } else { "D_ext" },
                    if idx < 360 { 8 } else { 2 },
                );
                sb.add_v9_compat_segments(
                    &v9_step2_prompt,
                    &v9_step2_segs,
                    if idx < 450 { "D" } else { "D_ext" },
                    if idx < 450 { 13 } else { 2 },
                );
                sb.add_v10_segments(&[], &v9_step2_prompt, &v9_step2_segs, "D_ext", 1);
            }
        }
    }

    // Direct `uname -a` & natural composer sandbox prompts (`gllm_v9_compositional.rs:285`)
    let uname_obs = "Linux mvm-wasm 6.6.0-uor-rv64imac #1 SMP PREEMPT_DYNAMIC riscv64 GNU/Linux";
    let uname_tail = &uname_obs[5..];
    let uname_step2_segs = [
        ("Verified sandbox output: Linux", false),
        (uname_tail, true),
        (".", false),
    ];
    let uname_agent_step2_segs = [
        ("MVM verified ok: Linux", false),
        (uname_tail, true),
        (".", false),
    ];
    sb.add_v9_compat_segments(
        &format!(
            "[agent_mode] Verify sandbox execution.\n<tool_response> {uname_obs}</tool_response>"
        ),
        &uname_agent_step2_segs,
        "D",
        45,
    );
    let uname_prompts = [
        "Run uname -a inside the MVM-Wasm Linux sandbox and report the architecture.",
        "Run uname -a inside the Linux MicroVM sandbox.",
        "Check uname -srm in the RISC-V MicroVM.",
        "Execute uname -a inside the MVM-Wasm Linux sandbox and report the architecture.",
        "Run `uname -a` inside the Linux MicroVM sandbox.",
    ];
    for (u_idx, &uname_p) in uname_prompts.iter().enumerate() {
        let tc_u = mvm_tc_fixed("uname -a");
        sb.add_v10_plain(&[], uname_p, &tc_u, "D", if u_idx == 0 { 350 } else { 150 });
        if uname_p != "Run uname -a inside the Linux MicroVM sandbox." {
            sb.add_v9_compat_segments(
                &format!("[agent_mode] {uname_p}"),
                &[(tc_u.as_str(), false)],
                "D",
                if u_idx == 0 { 190 } else { 60 },
            );
        }
        sb.add_v10_segments(
            &[],
            &format!(
                "Summarize the verified tool output for `{uname_p}`:\n<tool_response> {uname_obs}</tool_response>"
            ),
            &uname_step2_segs,
            "D",
            if u_idx < 3 { 40 } else { 12 },
        );
    }

    // 2. All 10 MVM static packages (`jq`, `rg`, `tree`, `xxd`, `sha256sum`, `bc`, `wc`, `head`, `tail`, `http`) + `pkg`
    let pkg_names = [
        "jq",
        "rg",
        "tree",
        "xxd",
        "sha256sum",
        "bc",
        "wc",
        "head",
        "tail",
        "http",
    ];
    for &pkg in &pkg_names {
        let tc_pkg = format!(
            "<tool_call>{{\"name\":\"mvm_exec\",\"arguments\":{{\"env\":\"mvm\",\"cmd\":\"pkg install {pkg}\"}}}}</tool_call>"
        );
        let pkg_tmpls = [
            format!("Install {pkg} inside the Linux MicroVM."),
            format!("Install package {pkg} inside the Linux MicroVM."),
            format!("Install the {pkg} binary inside the Linux MicroVM."),
            format!("Run pkg install {pkg} inside the sandbox."),
            format!("Can you install {pkg} inside the MVM?"),
        ];
        for (t_idx, tmpl) in pkg_tmpls.iter().enumerate() {
            if pkg == "jq" && t_idx == 0 {
                sb.add_v10_plain(&[], tmpl, &tc_pkg, "D", 340);
            } else if pkg == "jq" {
                sb.add_v10_plain(&[], tmpl, &tc_pkg, "D", 48);
            } else {
                let bkt = if t_idx < 2 { "D" } else { "D_ext" };
                let boost = if t_idx == 0 {
                    64
                } else if t_idx == 1 {
                    14
                } else {
                    6
                };
                sb.add_v10_plain(&[], tmpl, &tc_pkg, bkt, boost);
            }
            let obs =
                format!("installed /usr/local/bin/{pkg} (ELF64 EM_RISCV=243, sha256 verified)");
            let obs_sp = format!(" {obs}");
            let pkg_step2_segs = [
                ("Verified sandbox execution result --", false),
                (obs_sp.as_str(), true),
                (".", false),
            ];
            sb.add_v10_segments(
                &[],
                &format!(
                    "Summarize the verified tool output for `{tmpl}`:\n<tool_response> {obs}</tool_response>"
                ),
                &pkg_step2_segs,
                if pkg == "jq" || t_idx == 0 { "D" } else { "D_ext" },
                if t_idx == 0 { 2 } else { 1 },
            );
        }
    }

    sb.add_v10_plain(
        &[],
        "List available packages in the MVM sandbox.",
        &mvm_tc_fixed("pkg list"),
        "D",
        32,
    );
    sb.add_v10_plain(
        &[],
        "What files are in `/workspace`?",
        &mvm_tc_fixed("tree -L 2 /workspace"),
        "D",
        24,
    );

    // 3. Git tool-calling (`git status`, `git clone`, `git diff`, `git log`, `git push`)
    let git_orgs = [
        "rust-lang",
        "tokio-rs",
        "bytecodealliance",
        "serde-rs",
        "cloudflare",
        "torvalds",
        "BurntSushi",
        "dtolnay",
        "rayon-rs",
        "hyperium",
        "Catalyst-OS",
    ];
    let git_repos = [
        "libc", "mio", "wasmtime", "json", "workerd", "uom", "ripgrep", "anyhow", "syn", "bytes",
        "tracing", "hyper", "quinn", "tower", "md-docs", "mdcat", "md5-fast", "docBook", "netBook",
        "sysBook", "handbook", "playbook", "cmd-cli", "cmdr",
    ];

    let status_tc = git_tc_fixed("status");
    for st_p in [
        "Check `git status` in `/workspace`.",
        "Run git status.",
        "Check git status in the workspace.",
        "What is the current `git status`?",
    ] {
        sb.add_v10_plain(&[], st_p, &status_tc, "D", 230);
    }
    sb.add_v10_plain(
        &[],
        "Show the current git diff.",
        &git_tc_fixed("diff"),
        "D",
        24,
    );
    sb.add_v10_plain(
        &[],
        "Show the latest git commits.",
        &git_tc_fixed("log -n 5"),
        "D",
        24,
    );
    sb.add_v10_plain(
        &[],
        "Push branch `main` to origin.",
        &git_tc_fixed("push origin main"),
        "D",
        24,
    );

    for i in 0..1400usize {
        let org = git_orgs[i % git_orgs.len()];
        let rep = if i >= 120 {
            format!("{}-{}", git_repos[i % git_repos.len()], i % 67)
        } else {
            git_repos[i % git_repos.len()].to_string()
        };
        let url = format!("https://github.com/{org}/{rep}.git");
        let mode = (i / git_orgs.len()) % 3;
        let u = match mode {
            0 => format!("Clone {url} into `/workspace`."),
            1 => format!("Clone repository {url} into `/workspace`."),
            _ => format!("Use git to clone {url} into `/workspace`."),
        };
        let sp_url = format!(" {url}");
        let segs = [
            (
                "<tool_call>{\"name\":\"git\",\"arguments\":{\"cmd\":\"clone",
                false,
            ),
            (sp_url.as_str(), true),
            ("\"}}</tool_call>", false),
        ];
        let sha_full = sha256_hex(url.as_bytes());
        let sha = &sha_full[..7];
        let obs = format!("Cloned {url} into-dir /workspace/{rep} at HEAD {sha} (synced 14 files)");
        let obs_sp = format!(" {obs}");
        let ans_segs = [
            ("Verified sandbox execution result --", false),
            (obs_sp.as_str(), true),
            (".", false),
        ];
        let is_eval = i >= 1340;
        let g_bkt = if is_eval || i < 460 { "D" } else { "D_ext" };
        let g_boost = if is_eval {
            1
        } else if i < 460 {
            if mode == 0 {
                12
            } else {
                4
            }
        } else {
            2
        };
        emit_routed_tool_pair(
            sb,
            eval_sb,
            is_eval,
            &ToolPair {
                prompt: &u,
                tc_segs: &segs,
                tc_bucket: g_bkt,
                tc_boost: g_boost,
                obs: &obs,
                summary_segs: &ans_segs,
                summary_bucket: if is_eval || i < 100 { "D" } else { "D_ext" },
                summary_boost: 1,
            },
        );
    }
}

/// Bucket E (~5,500): Orthogonal `web_search` and `web_fetch` `<tool_call>` + grounded `<tool_response>`.
pub fn generate_bucket_e_web_tools(sb: &mut SampleBuilder, eval_sb: &mut SampleBuilder) {
    let search_subjects: &[(&str, &str)] = &[
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
    ];

    let search_openers: &[&str] = &[
        "Search the web for {q}.",
        "Look up {q}.",
        "Can you search online for {q}?",
        "Find current documentation on {q}.",
        "Use web search to check {q}.",
    ];

    let search_ref_urls: &[&str] = &[
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
    ];

    for s_i in 0..1400usize {
        let (base_q, summary) = search_subjects[s_i % search_subjects.len()];
        let ref_url = search_ref_urls[s_i % search_ref_urls.len()];
        let q = if s_i < search_subjects.len() * search_openers.len() {
            base_q.to_string()
        } else if s_i % 5 != 0 {
            let qual = PLAIN_SEARCH_QUALS[(s_i / search_subjects.len()) % PLAIN_SEARCH_QUALS.len()];
            format!("{base_q} {qual}")
        } else if (s_i / 5) % 2 == 0 {
            format!("{base_q} v{}.{}", 1 + (s_i % 4), s_i % 9)
        } else {
            format!("{base_q} reference {s_i}")
        };
        let opener = search_openers[s_i % search_openers.len()];
        let u = opener.replace("{q}", &q);
        let q_sp = format!(" {q}");
        let segs = [
            (
                "<tool_call>{\"name\":\"web_search\",\"arguments\":{\"scope\":\"web\",\"query\":\"",
                false,
            ),
            (q_sp.as_str(), true),
            ("\"}}</tool_call>", false),
        ];
        let obs = format!("1. **{q}** ({ref_url}) — {summary}");
        let sum_sp = format!(" {summary}");
        let ans_segs = [
            ("Verified web search result —", false),
            (sum_sp.as_str(), true),
        ];
        let is_eval = s_i >= 1340;
        let boost_n = if !is_eval && opener == "Search the web for {q}." {
            24
        } else if !is_eval {
            5
        } else {
            1
        };
        let e_bkt = if is_eval || s_i < 520 { "E" } else { "E_ext" };
        emit_routed_tool_pair(
            sb,
            eval_sb,
            is_eval,
            &ToolPair {
                prompt: &u,
                tc_segs: &segs,
                tc_bucket: e_bkt,
                tc_boost: boost_n,
                obs: &obs,
                summary_segs: &ans_segs,
                summary_bucket: if is_eval || s_i < 100 { "E" } else { "E_ext" },
                summary_boost: if !is_eval && s_i < 100 { 4 } else { 1 },
            },
        );
    }

    let domains: &[&str] = &[
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
    ];
    let fetch_openers: &[&str] = &[
        "Fetch {url} and summarize it.",
        "Read {url} and summarize the section.",
        "Use `web_fetch` to inspect {url} and summarize key points.",
        "Fetch {url} and show a summary.",
    ];
    let fetch_slugs: &[&str] = &[
        "index",
        "overview",
        "guide",
        "core",
        "types",
        "traits",
        "io",
        "spec",
        "reference",
        "module",
    ];
    let fetch_crates: &[&str] = &[
        "tokio",
        "serde",
        "tracing",
        "bytes",
        "rayon",
        "strum",
        "uom",
        "aurum",
        "quantum",
        "spectrum",
        "momentum",
        "hyper",
        "tower",
        "quinn",
        "clap",
        "anyhow",
        "thiserror",
        "nom",
        "crossbeam",
        "mio",
        "tonic",
        "syn",
        "futures",
        "semver",
    ];

    for f_i in 0..1400usize {
        let dom = domains[f_i % domains.len()];
        let url = if f_i < domains.len() * fetch_openers.len() {
            format!("https://{dom}")
        } else if f_i % 2 == 0 {
            let cr_f = format!("{}_{}", fetch_crates[f_i % fetch_crates.len()], f_i);
            format!("https://docs.rs/{cr_f}/latest/{cr_f}")
        } else {
            format!(
                "https://{dom}/{}_{}",
                fetch_slugs[f_i % fetch_slugs.len()],
                f_i
            )
        };
        let opener = fetch_openers[f_i % fetch_openers.len()];
        let u = opener.replace("{url}", &url);
        let url_sp = format!(" {url}");
        let segs = [
            (
                "<tool_call>{\"name\":\"web_fetch\",\"arguments\":{\"extract\":\"summary\",\"url\":\"",
                false,
            ),
            (url_sp.as_str(), true),
            ("\"}}</tool_call>", false),
        ];
        let obs = format!("HTTP 200 OK ({url}): Technical reference covering API invariants.");
        let obs_sp = format!(" {obs}");
        let ans_segs = [
            ("Verified sandbox execution result --", false),
            (obs_sp.as_str(), true),
        ];
        let is_eval = f_i >= 1340;
        let boost_n = if !is_eval
            && f_i < domains.len() * fetch_openers.len()
            && opener == "Fetch {url} and summarize it."
        {
            21
        } else if !is_eval && opener == "Fetch {url} and summarize it." {
            10
        } else if !is_eval {
            4
        } else {
            1
        };
        let ef_bkt = if is_eval || f_i < 380 { "E" } else { "E_ext" };
        emit_routed_tool_pair(
            sb,
            eval_sb,
            is_eval,
            &ToolPair {
                prompt: &u,
                tc_segs: &segs,
                tc_bucket: ef_bkt,
                tc_boost: boost_n,
                obs: &obs,
                summary_segs: &ans_segs,
                summary_bucket: if is_eval || f_i < 80 { "E" } else { "E_ext" },
                summary_boost: 1,
            },
        );
    }
}

/// Fill cleanly to `TARGET_TRAIN_COUNT = 100,000` with balanced high-SNR examples across Buckets A/C/D/E.
pub fn top_up_to_target(sb: &mut SampleBuilder, rng: &mut Rng64) {
    let prefixes: &[&str] = &[
        "alpha", "delta", "kappa", "sigma", "omega", "zeta", "lambda", "gamma", "matrix", "tensor",
        "stride", "window", "cursor", "packet", "frame", "sector", "inode", "digest", "quota",
        "margin", "scalar", "vertex", "anchor", "bucket", "origin", "orbit", "batch", "baryon",
        "nexus", "lattice", "cascade", "axel", "chain", "grain", "macro", "retro", "hydro",
        "micro",
    ];
    let mids: &[&str] = &[
        "count", "limit", "index", "scale", "shift", "delta", "score", "value", "width", "depth",
        "bound", "state",
    ];
    let suffixes: &[&str] = &[
        "a1", "b2", "c3", "d4", "e5", "f6", "g8", "h1", "j3", "k9", "m2", "m4", "n5", "p7", "q3",
        "r2", "t8", "u6", "v4", "w9", "x2", "y4", "z7", "z9",
    ];
    let crates: &[&str] = &[
        "hyper",
        "rayon",
        "anyhow",
        "thiserror",
        "clap",
        "nom",
        "crossbeam",
        "mio",
        "tower",
        "tonic",
        "serde",
        "tokio",
        "tracing",
        "bytes",
        "syn",
        "futures",
        "semver",
        "strum",
        "uom",
        "aurum",
        "quantum",
        "spectrum",
        "momentum",
    ];
    let git_repos_top: Vec<&str> = crates
        .iter()
        .copied()
        .chain(["md-docs", "mdcat", "md5-fast", "cmd-cli", "cmdr"])
        .collect();
    let git_orgs_top: &[&str] = &[
        "rust-lang",
        "tokio-rs",
        "bytecodealliance",
        "serde-rs",
        "cloudflare",
        "BurntSushi",
        "dtolnay",
        "rayon-rs",
        "hyperium",
        "Catalyst-OS",
    ];
    let search_topics_top: &[&str] = &[
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
    ];
    let thought_repo_top = "<thought>kv recall</thought>\nYour repository is called:";
    let thought_said_top = "<thought>prior turn</thought>\nEarlier in this chat you said:";
    let repo_ack_top = "Noted your saved name here for you.";
    let repo_prefixes_top: Vec<&str> = prefixes
        .iter()
        .copied()
        .filter(|p| !p.starts_with('a'))
        .collect();

    let fn_verbs_top: &[&str] = &[
        "combine", "clamp", "step", "mix", "fold", "scale", "merge", "align", "diff", "accum",
    ];
    let fn_nouns_top: &[&str] = &[
        "pair", "sum", "delta", "lanes", "window", "budget", "mux", "crux", "luxe", "radix",
        "plume",
    ];
    let repo_asks_top: &[&str] = &[
        "what is my repo called?",
        "What is my repository called?",
        "what did I say my repo was called?",
        "remind me what my project is called?",
    ];
    let verbs_said = [
        "auditing",
        "inspecting",
        "verifying",
        "checking",
        "tracing",
        "debugging",
    ];
    let ask_s_options = [
        "what did I just say?",
        "What did I just say?",
        "remind me what I just said",
    ];

    let mut idx = 0usize;
    while sb.records.len() < TARGET_TRAIN_COUNT && idx < 260_000 {
        let kind = idx % 10;
        idx += 1;
        let a = fresh_ident_unchecked(rng, prefixes, mids, suffixes);
        let b = fresh_ident_unchecked(rng, prefixes, mids, suffixes);
        if a == b || contains_forbidden(&a) || contains_forbidden(&b) {
            continue;
        }
        let va = 11 + (idx % 83);
        let vb = 3 + ((idx * 7) % 41);

        match kind {
            0 => {
                let p = format!(
                    "Given {a} = {va} and {b} = {vb}, write a concise Rust expression combining {a} and {b}."
                );
                add_rust_add_expr(sb, &[], &p, &a, &b, "C_top", 2);
            }
            1 => {
                if (idx / 10) % 2 == 0 {
                    let p = format!("Given {a} = {va} and {b} = {vb}, compute {a} + {b} in Rust.");
                    add_rust_add_expr(sb, &[], &p, &a, &b, "C_top", 3);
                } else {
                    let fn_c_t = format!(
                        "{}_{}",
                        fn_verbs_top[idx % fn_verbs_top.len()],
                        fn_nouns_top[(idx / 5) % fn_nouns_top.len()]
                    );
                    if contains_forbidden(&fn_c_t) {
                        continue;
                    }
                    let u_c_t = format!(
                        "Write a freestanding C function {fn_c_t}(long {a}) for the RV64IMAC MicroVM."
                    );
                    let _ = add_c_rv64_fn(sb, &[], &u_c_t, &fn_c_t, &a, "C_top", 2);
                }
            }
            2 => {
                let fn_name = format!(
                    "{}_{}_{}",
                    fn_verbs_top[idx % fn_verbs_top.len()],
                    fn_nouns_top[(idx / 3) % fn_nouns_top.len()],
                    suffixes[idx % suffixes.len()]
                );
                if contains_forbidden(&fn_name) {
                    continue;
                }
                let p_fn = format!("Write a Rust function {fn_name}( {a}: i64, {b}: i64 ) -> i64.");
                let _ = add_rust_sat_fn(sb, &[], &p_fn, &fn_name, &a, &b, "C_top", 6);
            }
            3 => {
                if (idx / 10) % 2 == 1 {
                    continue;
                }
                let repo_slug =
                    format!("{}-{}", *rng.choose(&repo_prefixes_top), *rng.choose(mids));
                if contains_forbidden(&repo_slug) {
                    continue;
                }
                let u1 = format!("my repo is called {repo_slug}");
                let warm_top = [
                    ("hi", REPLY_HI),
                    ("how are you?", REPLY_HOW_ARE_YOU),
                    ("who are you?", REPLY_WHO),
                ];
                sb.add_v10_plain(&warm_top, &u1, repo_ack_top, "A_top", 2);
                let a1 = if idx % 3 != 2 {
                    repo_ack_top.to_string()
                } else {
                    format!("Got it - tracking repository {repo_slug}.")
                };
                let hist_r_full = [
                    ("hi", REPLY_HI),
                    ("how are you?", REPLY_HOW_ARE_YOU),
                    ("who are you?", REPLY_WHO),
                    (u1.as_str(), a1.as_str()),
                ];
                let hist_r: &[(&str, &str)] = if idx % 3 != 0 {
                    &hist_r_full
                } else {
                    &hist_r_full[3..]
                };
                let slug_sp = format!(" {repo_slug}");
                let segs_repo = [
                    (thought_repo_top, false),
                    (slug_sp.as_str(), true),
                    (".", false),
                ];
                sb.add_v10_segments(
                    hist_r,
                    repo_asks_top[(idx / 10) % repo_asks_top.len()],
                    &segs_repo,
                    "A_top",
                    1,
                );
                let recall_ans = format!("Your repository is called: {repo_slug}.");
                let hist_5_top = [
                    ("hi", REPLY_HI),
                    ("how are you?", REPLY_HOW_ARE_YOU),
                    ("who are you?", REPLY_WHO),
                    (u1.as_str(), repo_ack_top),
                    ("what is my repo called?", recall_ans.as_str()),
                ];
                sb.add_v10_plain(
                    &hist_5_top,
                    "can you explain that simpler?",
                    "Sure - our causal KV cache keeps that saved repo label handy across turns.",
                    "A_top",
                    1,
                );
            }
            4 => {
                let vs = verbs_said[(idx / 10) % verbs_said.len()];
                let st_b = format!("{vs} the vsock-{} bounds", mids[idx % mids.len()]);
                let u1_s = format!("I am {st_b}.");
                let ack_s = "Got it - ready to help with your question.";
                sb.add_v10_plain(&[], &u1_s, ack_s, "A_top", 2);
                let ask_s = ask_s_options[(idx / 10) % ask_s_options.len()];
                let st_sp = format!(" {st_b}");
                let segs_said = [
                    (thought_said_top, false),
                    (st_sp.as_str(), true),
                    (".", false),
                ];
                sb.add_v10_segments(&[(u1_s.as_str(), ack_s)], ask_s, &segs_said, "A_top", 3);
            }
            5 => {
                let tag = if (idx / 20) % 2 == 0 {
                    format!("check-{a}-mvm")
                } else {
                    format!("check-{a}-{}", idx % 997)
                };
                let cmd_phrase = format!("uname -a and compute echo -n {tag} | sha256sum");
                let u_p = format!("Run {cmd_phrase} inside the Linux MicroVM sandbox.");
                let span = format!(" {tag} | sha256sum");
                let segs = [
                    (
                        "<tool_call>{\"name\":\"mvm_exec\",\"arguments\":{\"cmd\":\"uname -a && echo -n",
                        false,
                    ),
                    (span.as_str(), true),
                    ("\"}}</tool_call>", false),
                ];
                if (idx / 10) % 2 == 0 {
                    sb.add_v10_segments(&[], &u_p, &segs, "D_top", 1);
                } else {
                    sb.add_v9_compat_segments(&format!("[agent_mode] {u_p}"), &segs, "D_top", 2);
                }
            }
            6 => {
                let tag = format!("step2-{a}-{}", idx % 997);
                let dig_t = sha256_hex(tag.as_bytes());
                let obs_t_tail = format!(
                    " mvm-wasm 6.6.0-uor-rv64imac #1 SMP PREEMPT_DYNAMIC riscv64 GNU/Linux | {dig_t}  -"
                );
                let p_s2 = format!(
                    "[agent_mode] Verify sandbox execution.\n<tool_response> Linux{obs_t_tail}</tool_response>"
                );
                let segs_s2 = [
                    ("MVM verified ok: Linux", false),
                    (obs_t_tail.as_str(), true),
                    (".", false),
                ];
                sb.add_v9_compat_segments(&p_s2, &segs_s2, "D_top", 1);
            }
            7 => {
                let cr = format!("{}_{}", crates[(idx / 10) % crates.len()], idx % 499);
                let url = format!("https://docs.rs/{cr}/latest/{cr}");
                let u_f = format!("Fetch {url} and summarize it.");
                let url_sp = format!(" {url}");
                let segs = [
                    (
                        "<tool_call>{\"name\":\"web_fetch\",\"arguments\":{\"extract\":\"summary\",\"url\":\"",
                        false,
                    ),
                    (url_sp.as_str(), true),
                    ("\"}}</tool_call>", false),
                ];
                sb.add_v10_segments(&[], &u_f, &segs, "E_top", 2);
            }
            8 => {
                let org_t = git_orgs_top[(idx / 10) % git_orgs_top.len()];
                let cr = git_repos_top[(idx / 10) % git_repos_top.len()];
                let url_g = format!("https://github.com/{org_t}/{cr}-{}.git", idx % 499);
                let u_g = if (idx / 10) % 2 == 0 {
                    format!("Clone {url_g} into `/workspace`.")
                } else {
                    format!("Clone repository {url_g} into `/workspace`.")
                };
                let url_sp = format!(" {url_g}");
                let segs = [
                    (
                        "<tool_call>{\"name\":\"git\",\"arguments\":{\"cmd\":\"clone",
                        false,
                    ),
                    (url_sp.as_str(), true),
                    ("\"}}</tool_call>", false),
                ];
                sb.add_v10_segments(&[], &u_g, &segs, "D_top", 1);
            }
            _ => {
                let k9_step = (idx - 1) / 10;
                let topic_s = search_topics_top[k9_step % search_topics_top.len()];
                let (q_s, u_s, k9_boost) = if k9_step % 5 != 0 {
                    let qual_s = PLAIN_SEARCH_QUALS
                        [(k9_step / search_topics_top.len()) % PLAIN_SEARCH_QUALS.len()];
                    let mut q_s = format!("{topic_s} {qual_s}");
                    let cycle_k9 = k9_step / (search_topics_top.len() * PLAIN_SEARCH_QUALS.len());
                    if cycle_k9 >= 2 {
                        q_s = format!("{q_s} note {cycle_k9}");
                    }
                    let op_k9 = if cycle_k9 == 0 {
                        if k9_step % 3 != 2 {
                            "Search the web for {q}."
                        } else if (k9_step / 3) % 2 == 0 {
                            "Look up {q} on the web."
                        } else {
                            "Run web_search for {q}."
                        }
                    } else if k9_step % 3 != 2 {
                        if k9_step % 2 == 0 {
                            "Look up {q} on the web."
                        } else {
                            "Run web_search for {q}."
                        }
                    } else {
                        "Search the web for {q}."
                    };
                    let u_s = op_k9.replace("{q}", &q_s);
                    (q_s, u_s, 4usize)
                } else {
                    let q_s = format!("{topic_s} {}.{}", 1 + (k9_step % 9), k9_step % 19);
                    let u_s = format!("Search the web for {q_s}.");
                    (q_s, u_s, 3usize)
                };
                if contains_forbidden(&q_s) {
                    continue;
                }
                let q_sp = format!(" {q_s}");
                let segs = [
                    (
                        "<tool_call>{\"name\":\"web_search\",\"arguments\":{\"scope\":\"web\",\"query\":\"",
                        false,
                    ),
                    (q_sp.as_str(), true),
                    ("\"}}</tool_call>", false),
                ];
                sb.add_v10_segments(&[], &u_s, &segs, "E_top", k9_boost);
            }
        }
    }
}

fn fresh_ident_unchecked(
    rng: &mut Rng64,
    prefixes: &[&str],
    mids: &[&str],
    suffixes: &[&str],
) -> String {
    let p = *rng.choose(prefixes);
    let m = *rng.choose(mids);
    let s = *rng.choose(suffixes);
    format!("{p}_{m}_{s}")
}
