//! Deterministically generated Rust implementation of the verified LexLean `R4.*` modules.
//!
//! Generated directly from the `\semanticdata{...}` ASTs in `src/R4/*.lex.tex`.
//! Bound to `public/provenance.json["generated_core_sha256"]`.

#![allow(
    clippy::all,
    unused_parens,
    unused_variables,
    unused_comparisons,
    dead_code,
    non_snake_case
)]

/// Generated from `R4.Types` (`src/R4/Types.lex.tex`).
pub mod r4_types {
    /// Structure `R4.Types.Spec`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Spec {
        /// Field `vocabulary`.
        pub vocabulary: u64,
        /// Field `blocks`.
        pub blocks: u64,
        /// Field `context`.
        pub context: u64,
        /// Field `keptCells`.
        pub kept_cells: u64,
    }

    /// Function `R4.Types.inRange`.
    pub fn in_range(minimum: u64, value: u64, maximum: u64) -> bool {
        (((minimum.clone()) <= (value.clone())) && ((value.clone()) <= (maximum.clone())))
    }

    /// Function `R4.Types.validSpec`.
    pub fn valid_spec(vocabulary: u64, blocks: u64, context: u64, kept_cells: u64) -> bool {
        ((in_range(2u64, vocabulary.clone(), 8192u64))
            && ((in_range(2u64, blocks.clone(), 16u64))
                && ((in_range(1u64, context.clone(), 243u64))
                    && ((in_range(1u64, kept_cells.clone(), 16u64))
                        && ((kept_cells.clone()) <= (vocabulary.clone()))))))
    }
}

/// Generated from `R4.Objective` (`src/R4/Objective.lex.tex`).
pub mod r4_objective {
    /// Inductive type `R4.Objective.ObjectivePolicy`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ObjectivePolicy {
        /// Variant `nearestAddress`.
        NearestAddress,
        /// Variant `addressRank`.
        AddressRank,
    }

    /// Inductive type `R4.Objective.WeightedOutcome`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum WeightedOutcome {
        /// Variant `value`.
        Value(u64),
        /// Variant `arithmeticOverflow`.
        ArithmeticOverflow,
        /// Variant `invalidOperand`.
        InvalidOperand,
    }

    /// Function `R4.Objective.predecessor`.
    pub fn predecessor(value: u64) -> u64 {
        {
            let __scrut = value.clone();
            if __scrut == 0 {
                0u64
            } else {
                let prior = __scrut - 1;
                prior.clone()
            }
        }
    }

    /// Function `R4.Objective.subtractNat`.
    pub fn subtract_nat(left: u64, right: u64) -> u64 {
        {
            let __scrut = right.clone();
            if __scrut == 0 {
                left.clone()
            } else {
                let prior = __scrut - 1;
                subtract_nat(predecessor(left.clone()), prior.clone())
            }
        }
    }

    /// Function `R4.Objective.scaleAccumulator`.
    pub fn scale_accumulator(value: u64, times: u64, total: u64) -> u64 {
        {
            let __scrut = times.clone();
            if __scrut == 0 {
                total.clone()
            } else {
                let prior = __scrut - 1;
                scale_accumulator(
                    value.clone(),
                    prior.clone(),
                    (total.clone()).saturating_add(value.clone()),
                )
            }
        }
    }

    /// Function `R4.Objective.scaleChunks`.
    pub fn scale_chunks(value: u64, times: u64, total: u64, fuel: u64) -> u64 {
        {
            let __scrut = fuel.clone();
            if __scrut == 0 {
                scale_accumulator(value.clone(), times.clone(), total.clone())
            } else {
                let fuel_prior = __scrut - 1;
                {
                    let __scrut = times.clone();
                    if __scrut == 0 {
                        total.clone()
                    } else {
                        let rest0 = __scrut - 1;
                        {
                            let __scrut = rest0.clone();
                            if __scrut == 0 {
                                (total.clone()).saturating_add(value.clone())
                            } else {
                                let rest1 = __scrut - 1;
                                {
                                    let __scrut = rest1.clone();
                                    if __scrut == 0 {
                                        (total.clone()).saturating_add(
                                            (value.clone()).saturating_add(value.clone()),
                                        )
                                    } else {
                                        let rest2 = __scrut - 1;
                                        {
                                            let __scrut = rest2.clone();
                                            if __scrut == 0 {
                                                (total.clone()).saturating_add(
                                                    ((value.clone()).saturating_add(value.clone()))
                                                        .saturating_add(value.clone()),
                                                )
                                            } else {
                                                let rest3 = __scrut - 1;
                                                {
                                                    let __scrut = rest3.clone();
                                                    if __scrut == 0 {
                                                        (total.clone()).saturating_add(
                                                            (((value.clone())
                                                                .saturating_add(value.clone()))
                                                            .saturating_add(value.clone()))
                                                            .saturating_add(value.clone()),
                                                        )
                                                    } else {
                                                        let rest4 = __scrut - 1;
                                                        {
                                                            let __scrut = rest4.clone();
                                                            if __scrut == 0 {
                                                                (total.clone()).saturating_add(((((value.clone()).saturating_add(value.clone())).saturating_add(value.clone())).saturating_add(value.clone())).saturating_add(value.clone()))
                                                            } else {
                                                                let rest5 = __scrut - 1;
                                                                {
                                                                    let __scrut = rest5.clone();
                                                                    if __scrut == 0 {
                                                                        (total.clone())
                                                                            .saturating_add(
                                                                                (((((value
                                                                                    .clone())
                                                                                .saturating_add(
                                                                                    value.clone(),
                                                                                ))
                                                                                .saturating_add(
                                                                                    value.clone(),
                                                                                ))
                                                                                .saturating_add(
                                                                                    value.clone(),
                                                                                ))
                                                                                .saturating_add(
                                                                                    value.clone(),
                                                                                ))
                                                                                .saturating_add(
                                                                                    value.clone(),
                                                                                ),
                                                                            )
                                                                    } else {
                                                                        let rest6 = __scrut - 1;
                                                                        {
                                                                            let __scrut =
                                                                                rest6.clone();
                                                                            if __scrut == 0 {
                                                                                (total.clone()).saturating_add(((((((value.clone()).saturating_add(value.clone())).saturating_add(value.clone())).saturating_add(value.clone())).saturating_add(value.clone())).saturating_add(value.clone())).saturating_add(value.clone()))
                                                                            } else {
                                                                                let rest7 =
                                                                                    __scrut - 1;
                                                                                scale_chunks(value.clone(), rest7.clone(), (total.clone()).saturating_add((((((((value.clone()).saturating_add(value.clone())).saturating_add(value.clone())).saturating_add(value.clone())).saturating_add(value.clone())).saturating_add(value.clone())).saturating_add(value.clone())).saturating_add(value.clone())), fuel_prior.clone())
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Function `R4.Objective.scaleNat`.
    pub fn scale_nat(value: u64, times: u64) -> u64 {
        scale_chunks(value.clone(), times.clone(), 0u64, times.clone())
    }

    /// Function `R4.Objective.hinge`.
    pub fn hinge(
        margin: u64,
        target_distance: u64,
        rival_distance: u64,
        rival_absent: bool,
    ) -> u64 {
        if rival_absent.clone() {
            0u64
        } else {
            subtract_nat(
                (margin.clone()).saturating_add(target_distance.clone()),
                rival_distance.clone(),
            )
        }
    }

    /// Function `R4.Objective.rankViolation`.
    pub fn rank_violation(rank: u64, kept_cells: u64) -> u64 {
        subtract_nat(rank.clone(), kept_cells.clone())
    }

    /// Function `R4.Objective.selectObjective`.
    pub fn select_objective(policy: ObjectivePolicy, nearest_cost: u64, rank_cost: u64) -> u64 {
        match policy.clone() {
            ObjectivePolicy::NearestAddress => nearest_cost.clone(),
            ObjectivePolicy::AddressRank => rank_cost.clone(),
        }
    }

    /// Function `R4.Objective.combineWeights`.
    pub fn combine_weights(
        token_weight: u64,
        address_weight: u64,
        rank_weight: u64,
        policy: ObjectivePolicy,
    ) -> WeightedOutcome {
        if (((18446744073709551615u64) <= (token_weight.clone()))
            || (((18446744073709551615u64) <= (address_weight.clone()))
                || (((18446744073709551615u64) <= (rank_weight.clone()))
                    || (((18446744073709551615u64)
                        < ((token_weight.clone()).saturating_add(address_weight.clone())))
                        || ((18446744073709551615u64)
                            < ((token_weight.clone()).saturating_add(rank_weight.clone())))))))
        {
            WeightedOutcome::ArithmeticOverflow
        } else {
            WeightedOutcome::Value(select_objective(
                policy.clone(),
                (token_weight.clone()).saturating_add(address_weight.clone()),
                (token_weight.clone()).saturating_add(rank_weight.clone()),
            ))
        }
    }

    /// Function `R4.Objective.weightedObjective`.
    pub fn weighted_objective(
        token_hinge: u64,
        address_hinge: u64,
        rank_miss: u64,
        width: u64,
        policy: ObjectivePolicy,
    ) -> WeightedOutcome {
        if (((token_hinge.clone()) <= (18446744073709551615u64))
            && (((address_hinge.clone()) <= (18446744073709551615u64))
                && (((rank_miss.clone()) <= (18446744073709551615u64))
                    && ((width.clone()) <= (4294967295u64)))))
        {
            combine_weights(
                scale_nat(token_hinge.clone(), 8u64),
                scale_nat(address_hinge.clone(), width.clone()),
                scale_nat(rank_miss.clone(), width.clone()),
                policy.clone(),
            )
        } else {
            WeightedOutcome::InvalidOperand
        }
    }
}

/// Generated from `R4.Configuration` (`src/R4/Configuration.lex.tex`).
pub mod r4_configuration {
    /// Inductive type `R4.Configuration.ReadSpec`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ReadSpec {
        /// Variant `route`.
        Route,
        /// Variant `flock`.
        Flock(u64),
        /// Variant `relation`.
        Relation(u64, u64),
    }

    /// Inductive type `R4.Configuration.ReadInitialization`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ReadInitialization {
        /// Variant `zeroGatesV1`.
        ZeroGatesV1,
        /// Variant `activeLeafV1`.
        ActiveLeafV1,
    }

    /// Inductive type `R4.Configuration.Validation`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Validation {
        /// Variant `accepted`.
        Accepted,
        /// Variant `invalidSpec`.
        InvalidSpec,
        /// Variant `outsideWord`.
        OutsideWord,
        /// Variant `invalidFlockKept`.
        InvalidFlockKept,
        /// Variant `invalidRelationKept`.
        InvalidRelationKept,
        /// Variant `invalidRelationSweeps`.
        InvalidRelationSweeps,
        /// Variant `activeRoute`.
        ActiveRoute,
    }

    /// Inductive type `R4.Configuration.GateOutcome`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum GateOutcome {
        /// Variant `count`.
        Count(u64),
        /// Variant `overflow`.
        Overflow,
        /// Variant `outsideWord`.
        OutsideWord,
    }

    /// Function `R4.Configuration.isWord`.
    pub fn is_word(value: u64) -> bool {
        ((value.clone()) <= (4294967295u64))
    }

    /// Function `R4.Configuration.readWordsAdmitted`.
    pub fn read_words_admitted(read: ReadSpec) -> bool {
        match read.clone() {
            ReadSpec::Route => true,
            ReadSpec::Flock(kept) => is_word(kept.clone()),
            ReadSpec::Relation(kept, sweeps) => {
                ((is_word(kept.clone())) && (is_word(sweeps.clone())))
            }
        }
    }

    /// Function `R4.Configuration.validateReadBounds`.
    pub fn validate_read_bounds(read: ReadSpec) -> Validation {
        match read.clone() {
            ReadSpec::Route => Validation::Accepted,
            ReadSpec::Flock(kept) => {
                if super::r4_types::in_range(1u64, kept.clone(), 8u64) {
                    Validation::Accepted
                } else {
                    Validation::InvalidFlockKept
                }
            }
            ReadSpec::Relation(kept, sweeps) => {
                if super::r4_types::in_range(1u64, kept.clone(), 8u64) {
                    if super::r4_types::in_range(1u64, sweeps.clone(), 5u64) {
                        Validation::Accepted
                    } else {
                        Validation::InvalidRelationSweeps
                    }
                } else {
                    Validation::InvalidRelationKept
                }
            }
        }
    }

    /// Function `R4.Configuration.validateRead`.
    pub fn validate_read(spec: super::r4_types::Spec, read: ReadSpec) -> Validation {
        if super::r4_types::valid_spec(
            (spec.clone()).vocabulary.clone(),
            (spec.clone()).blocks.clone(),
            (spec.clone()).context.clone(),
            (spec.clone()).kept_cells.clone(),
        ) {
            if read_words_admitted(read.clone()) {
                validate_read_bounds(read.clone())
            } else {
                Validation::OutsideWord
            }
        } else {
            Validation::InvalidSpec
        }
    }

    /// Function `R4.Configuration.words`.
    pub fn words(read: ReadSpec) -> Vec<u64> {
        match read.clone() {
            ReadSpec::Route => {
                let mut __v = vec![0u64];
                __v.extend({
                    let mut __v = vec![0u64];
                    __v.extend({
                        let mut __v = vec![0u64];
                        __v.extend(Vec::<u64>::new());
                        __v
                    });
                    __v
                });
                __v
            }
            ReadSpec::Flock(kept) => {
                let mut __v = vec![1u64];
                __v.extend({
                    let mut __v = vec![kept.clone()];
                    __v.extend({
                        let mut __v = vec![0u64];
                        __v.extend(Vec::<u64>::new());
                        __v
                    });
                    __v
                });
                __v
            }
            ReadSpec::Relation(kept, sweeps) => {
                let mut __v = vec![2u64];
                __v.extend({
                    let mut __v = vec![kept.clone()];
                    __v.extend({
                        let mut __v = vec![sweeps.clone()];
                        __v.extend(Vec::<u64>::new());
                        __v
                    });
                    __v
                });
                __v
            }
        }
    }

    /// Function `R4.Configuration.isZeroGates`.
    pub fn is_zero_gates(initialization: ReadInitialization) -> bool {
        match initialization.clone() {
            ReadInitialization::ZeroGatesV1 => true,
            ReadInitialization::ActiveLeafV1 => false,
        }
    }

    /// Function `R4.Configuration.validateInitialization`.
    pub fn validate_initialization(
        initialization: ReadInitialization,
        read: ReadSpec,
    ) -> Validation {
        match initialization.clone() {
            ReadInitialization::ZeroGatesV1 => Validation::Accepted,
            ReadInitialization::ActiveLeafV1 => match read.clone() {
                ReadSpec::Route => Validation::ActiveRoute,
                ReadSpec::Flock(kept) => Validation::Accepted,
                ReadSpec::Relation(kept, sweeps) => Validation::Accepted,
            },
        }
    }

    /// Function `R4.Configuration.product`.
    pub fn product(left: u64, right: u64) -> u64 {
        if ((left.clone()) <= (right.clone())) {
            super::r4_objective::scale_nat(right.clone(), left.clone())
        } else {
            super::r4_objective::scale_nat(left.clone(), right.clone())
        }
    }

    /// Function `R4.Configuration.checkedCount`.
    pub fn checked_count(value: u64) -> GateOutcome {
        if is_word(value.clone()) {
            GateOutcome::Count(value.clone())
        } else {
            GateOutcome::Overflow
        }
    }

    /// Function `R4.Configuration.finishGateCount`.
    pub fn finish_gate_count(read: ReadSpec, base: u64, stages: u64) -> GateOutcome {
        if is_word(base.clone()) {
            match read.clone() {
                ReadSpec::Route => GateOutcome::Count(base.clone()),
                ReadSpec::Flock(kept) => GateOutcome::Count(base.clone()),
                ReadSpec::Relation(kept, sweeps) => {
                    if is_word(product(5u64, stages.clone())) {
                        checked_count((base.clone()).saturating_add(product(5u64, stages.clone())))
                    } else {
                        GateOutcome::Overflow
                    }
                }
            }
        } else {
            GateOutcome::Overflow
        }
    }

    /// Function `R4.Configuration.gateCount`.
    pub fn gate_count(read: ReadSpec, blocks: u64, stages: u64) -> GateOutcome {
        if ((read_words_admitted(read.clone()))
            && ((is_word(blocks.clone())) && (is_word(stages.clone()))))
        {
            finish_gate_count(
                read.clone(),
                product(blocks.clone(), stages.clone()),
                stages.clone(),
            )
        } else {
            GateOutcome::OutsideWord
        }
    }
}

/// Generated from `R4.Gates` (`src/R4/Gates.lex.tex`).
pub mod r4_gates {
    /// Inductive type `R4.Gates.Trit`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Trit {
        /// Variant `zero`.
        Zero,
        /// Variant `negative`.
        Negative,
        /// Variant `positive`.
        Positive,
    }

    /// Function `R4.Gates.gateOrdinalValid`.
    pub fn gate_ordinal_valid(ordinal: u64) -> bool {
        ((ordinal.clone()) < (3u64))
    }

    /// Function `R4.Gates.gateCandidate`.
    pub fn gate_candidate(current: Trit, ordinal: u64) -> Option<Trit> {
        if ((ordinal.clone()) == (0u64)) {
            Some(current.clone())
        } else {
            if gate_ordinal_valid(ordinal.clone()) {
                match current.clone() {
                    Trit::Zero => Some(if ((ordinal.clone()) == (1u64)) {
                        Trit::Negative
                    } else {
                        Trit::Positive
                    }),
                    Trit::Negative => Some(if ((ordinal.clone()) == (1u64)) {
                        Trit::Zero
                    } else {
                        Trit::Positive
                    }),
                    Trit::Positive => Some(if ((ordinal.clone()) == (1u64)) {
                        Trit::Zero
                    } else {
                        Trit::Negative
                    }),
                }
            } else {
                None::<Trit>
            }
        }
    }

    /// Function `R4.Gates.wireWord`.
    pub fn wire_word(value: Trit) -> u64 {
        match value.clone() {
            Trit::Zero => 0u64,
            Trit::Negative => 4294967295u64,
            Trit::Positive => 1u64,
        }
    }

    /// Function `R4.Gates.traceCode`.
    pub fn trace_code(value: Trit) -> u64 {
        match value.clone() {
            Trit::Zero => 1u64,
            Trit::Negative => 0u64,
            Trit::Positive => 2u64,
        }
    }

    /// Function `R4.Gates.fromWireWord`.
    pub fn from_wire_word(value: u64) -> Option<Trit> {
        if ((value.clone()) == (0u64)) {
            Some(Trit::Zero)
        } else {
            if ((value.clone()) == (1u64)) {
                Some(Trit::Positive)
            } else {
                if ((value.clone()) == (4294967295u64)) {
                    Some(Trit::Negative)
                } else {
                    None::<Trit>
                }
            }
        }
    }

    /// Function `R4.Gates.fromTraceCode`.
    pub fn from_trace_code(value: u64) -> Option<Trit> {
        if ((value.clone()) == (0u64)) {
            Some(Trit::Negative)
        } else {
            if ((value.clone()) == (1u64)) {
                Some(Trit::Zero)
            } else {
                if ((value.clone()) == (2u64)) {
                    Some(Trit::Positive)
                } else {
                    None::<Trit>
                }
            }
        }
    }
}

/// Generated from `R4.Binder` (`src/R4/Binder.lex.tex`).
pub mod r4_binder {
    /// Inductive type `R4.Binder.BinderError`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum BinderError {
        /// Variant `inventory`.
        Inventory,
        /// Variant `nodeShape`.
        NodeShape,
        /// Variant `childLink`.
        ChildLink,
        /// Variant `contextStages`.
        ContextStages,
        /// Variant `frontierRoot`.
        FrontierRoot,
        /// Variant `omittedRoot`.
        OmittedRoot,
        /// Variant `atomOrder`.
        AtomOrder,
        /// Variant `atomCoverage`.
        AtomCoverage,
    }

    /// Structure `R4.Binder.Triple`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Triple {
        /// Field `first`.
        pub first: u64,
        /// Field `second`.
        pub second: u64,
        /// Field `third`.
        pub third: u64,
    }

    /// Structure `R4.Binder.BinderNode`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct BinderNode {
        /// Field `children`.
        pub children: Option<Triple>,
        /// Field `atom`.
        pub atom: Option<u64>,
        /// Field `level`.
        pub level: u64,
    }

    /// Structure `R4.Binder.BinderPlan`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct BinderPlan {
        /// Field `context`.
        pub context: u64,
        /// Field `nodes`.
        pub nodes: Vec<BinderNode>,
        /// Field `frontier`.
        pub frontier: Vec<u64>,
        /// Field `stages`.
        pub stages: u64,
    }

    /// Inductive type `R4.Binder.Validation`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Validation {
        /// Variant `valid`.
        Valid,
        /// Variant `invalid`.
        Invalid(BinderError),
    }

    /// Inductive type `R4.Binder.Construction`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Construction {
        /// Variant `built`.
        Built(BinderPlan),
        /// Variant `invalidContext`.
        InvalidContext,
    }

    /// Structure `R4.Binder.Root`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Root {
        /// Field `index`.
        pub index: u64,
        /// Field `level`.
        pub level: u64,
    }

    /// Function `R4.Binder.incrementIndex`.
    pub fn increment_index(index: Vec<bool>) -> Vec<bool> {
        {
            let __scrut = index.clone();
            if __scrut.is_empty() {
                {
                    let mut __v = vec![true];
                    __v.extend(Vec::<bool>::new());
                    __v
                }
            } else {
                let bit = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                if bit.clone() {
                    {
                        let mut __v = vec![false];
                        __v.extend(increment_index(tail.clone()));
                        __v
                    }
                } else {
                    {
                        let mut __v = vec![true];
                        __v.extend(tail.clone());
                        __v
                    }
                }
            }
        }
    }

    /// Function `R4.Binder.indexNat`.
    pub fn index_nat(index: Vec<bool>) -> u64 {
        {
            let __scrut = index.clone();
            if __scrut.is_empty() {
                0u64
            } else {
                let bit = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                if bit.clone() {
                    ((index_nat(tail.clone())).saturating_add(index_nat(tail.clone())))
                        .saturating_add(1)
                } else {
                    (index_nat(tail.clone())).saturating_add(index_nat(tail.clone()))
                }
            }
        }
    }

    /// Inductive type `R4.Binder.BuildState`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum BuildState {
        /// Variant `state`.
        State(Vec<BinderNode>, Vec<bool>, Vec<Root>),
    }

    /// Function `R4.Binder.buildNodes`.
    pub fn build_nodes(state: BuildState) -> Vec<BinderNode> {
        match state.clone() {
            BuildState::State(nodes, count, frontier) => nodes.clone(),
        }
    }

    /// Function `R4.Binder.buildCount`.
    pub fn build_count(state: BuildState) -> Vec<bool> {
        match state.clone() {
            BuildState::State(nodes, count, frontier) => count.clone(),
        }
    }

    /// Function `R4.Binder.buildFrontier`.
    pub fn build_frontier(state: BuildState) -> Vec<Root> {
        match state.clone() {
            BuildState::State(nodes, count, frontier) => frontier.clone(),
        }
    }

    /// Structure `R4.Binder.ScanState`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ScanState {
        /// Field `nextAtom`.
        pub next_atom: u64,
        /// Field `maximumLevel`.
        pub maximum_level: u64,
        /// Field `used`.
        pub used: Vec<u64>,
    }

    /// Inductive type `R4.Binder.ScanResult`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ScanResult {
        /// Variant `scanned`.
        Scanned(ScanState),
        /// Variant `failed`.
        Failed(BinderError),
    }

    /// Function `R4.Binder.validContext`.
    pub fn valid_context(context: u64) -> bool {
        (((1u64) <= (context.clone())) && ((context.clone()) <= (243u64)))
    }

    /// Function `R4.Binder.lengthNat`.
    pub fn length_nat(items: Vec<u64>) -> u64 {
        {
            let __scrut = items.clone();
            if __scrut.is_empty() {
                0u64
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                (length_nat(tail.clone())).saturating_add(1)
            }
        }
    }

    /// Function `R4.Binder.appendNat`.
    pub fn append_nat(left: Vec<u64>, right: Vec<u64>) -> Vec<u64> {
        {
            let __scrut = left.clone();
            if __scrut.is_empty() {
                right.clone()
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                {
                    let mut __v = vec![head.clone()];
                    __v.extend(append_nat(tail.clone(), right.clone()));
                    __v
                }
            }
        }
    }

    /// Function `R4.Binder.countNodesLinear`.
    pub fn count_nodes_linear(items: Vec<BinderNode>, total: u64) -> u64 {
        {
            let __scrut = items.clone();
            if __scrut.is_empty() {
                total.clone()
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                count_nodes_linear(tail.clone(), (total.clone()).saturating_add(1))
            }
        }
    }

    /// Function `R4.Binder.countNodeChunks`.
    pub fn count_node_chunks(fuel: u64, items: Vec<BinderNode>, total: u64) -> u64 {
        {
            let __scrut = fuel.clone();
            if __scrut == 0 {
                count_nodes_linear(items.clone(), total.clone())
            } else {
                let fuel_prior = __scrut - 1;
                {
                    let __scrut = items.clone();
                    if __scrut.is_empty() {
                        total.clone()
                    } else {
                        let chunk_head0 = __scrut[0].clone();
                        let chunk_tail0 = __scrut[1..].to_vec();
                        {
                            let __scrut = chunk_tail0.clone();
                            if __scrut.is_empty() {
                                (total.clone()).saturating_add(1)
                            } else {
                                let chunk_head1 = __scrut[0].clone();
                                let chunk_tail1 = __scrut[1..].to_vec();
                                {
                                    let __scrut = chunk_tail1.clone();
                                    if __scrut.is_empty() {
                                        ((total.clone()).saturating_add(1)).saturating_add(1)
                                    } else {
                                        let chunk_head2 = __scrut[0].clone();
                                        let chunk_tail2 = __scrut[1..].to_vec();
                                        {
                                            let __scrut = chunk_tail2.clone();
                                            if __scrut.is_empty() {
                                                (((total.clone()).saturating_add(1))
                                                    .saturating_add(1))
                                                .saturating_add(1)
                                            } else {
                                                let chunk_head3 = __scrut[0].clone();
                                                let chunk_tail3 = __scrut[1..].to_vec();
                                                {
                                                    let __scrut = chunk_tail3.clone();
                                                    if __scrut.is_empty() {
                                                        ((((total.clone()).saturating_add(1))
                                                            .saturating_add(1))
                                                        .saturating_add(1))
                                                        .saturating_add(1)
                                                    } else {
                                                        let chunk_head4 = __scrut[0].clone();
                                                        let chunk_tail4 = __scrut[1..].to_vec();
                                                        {
                                                            let __scrut = chunk_tail4.clone();
                                                            if __scrut.is_empty() {
                                                                (((((total.clone())
                                                                    .saturating_add(1))
                                                                .saturating_add(1))
                                                                .saturating_add(1))
                                                                .saturating_add(1))
                                                                .saturating_add(1)
                                                            } else {
                                                                let chunk_head5 =
                                                                    __scrut[0].clone();
                                                                let chunk_tail5 =
                                                                    __scrut[1..].to_vec();
                                                                {
                                                                    let __scrut =
                                                                        chunk_tail5.clone();
                                                                    if __scrut.is_empty() {
                                                                        ((((((total.clone())
                                                                            .saturating_add(1))
                                                                        .saturating_add(1))
                                                                        .saturating_add(1))
                                                                        .saturating_add(1))
                                                                        .saturating_add(1))
                                                                        .saturating_add(1)
                                                                    } else {
                                                                        let chunk_head6 =
                                                                            __scrut[0].clone();
                                                                        let chunk_tail6 =
                                                                            __scrut[1..].to_vec();
                                                                        {
                                                                            let __scrut =
                                                                                chunk_tail6.clone();
                                                                            if __scrut.is_empty() {
                                                                                (((((((total.clone()).saturating_add(1)).saturating_add(1)).saturating_add(1)).saturating_add(1)).saturating_add(1)).saturating_add(1)).saturating_add(1)
                                                                            } else {
                                                                                let chunk_head7 =
                                                                                    __scrut[0]
                                                                                        .clone();
                                                                                let chunk_tail7 =
                                                                                    __scrut[1..]
                                                                                        .to_vec();
                                                                                count_node_chunks(fuel_prior.clone(), chunk_tail7.clone(), ((((((((total.clone()).saturating_add(1)).saturating_add(1)).saturating_add(1)).saturating_add(1)).saturating_add(1)).saturating_add(1)).saturating_add(1)).saturating_add(1))
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Function `R4.Binder.countNodes`.
    pub fn count_nodes(items: Vec<BinderNode>, total: u64) -> u64 {
        count_node_chunks(64u64, items.clone(), total.clone())
    }

    /// Function `R4.Binder.lengthNode`.
    pub fn length_node(items: Vec<BinderNode>) -> u64 {
        count_nodes(items.clone(), 0u64)
    }

    /// Function `R4.Binder.containsNat`.
    pub fn contains_nat(items: Vec<u64>, needle: u64) -> bool {
        {
            let __scrut = items.clone();
            if __scrut.is_empty() {
                false
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                (((head.clone()) == (needle.clone()))
                    || (contains_nat(tail.clone(), needle.clone())))
            }
        }
    }

    /// Function `R4.Binder.reverseRoots`.
    pub fn reverse_roots(items: Vec<Root>, result: Vec<u64>) -> Vec<u64> {
        {
            let __scrut = items.clone();
            if __scrut.is_empty() {
                result.clone()
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                reverse_roots(tail.clone(), {
                    let mut __v = vec![(head.clone()).index.clone()];
                    __v.extend(result.clone());
                    __v
                })
            }
        }
    }

    /// Function `R4.Binder.carry`.
    pub fn carry(
        frontier: Vec<Root>,
        nodes: Vec<BinderNode>,
        next_index: Vec<bool>,
        current: Root,
        pending: Option<Root>,
    ) -> BuildState {
        {
            let __scrut = frontier.clone();
            if __scrut.is_empty() {
                match pending.clone() {
                    None => BuildState::State(nodes.clone(), next_index.clone(), {
                        let mut __v = vec![current.clone()];
                        __v.extend(Vec::<Root>::new());
                        __v
                    }),
                    Some(older) => BuildState::State(nodes.clone(), next_index.clone(), {
                        let mut __v = vec![current.clone()];
                        __v.extend({
                            let mut __v = vec![older.clone()];
                            __v.extend(Vec::<Root>::new());
                            __v
                        });
                        __v
                    }),
                }
            } else {
                let oldest = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                match pending.clone() {
                    None => {
                        if (((oldest.clone()).level.clone()) == ((current.clone()).level.clone())) {
                            carry(
                                tail.clone(),
                                nodes.clone(),
                                next_index.clone(),
                                current.clone(),
                                Some(oldest.clone()),
                            )
                        } else {
                            BuildState::State(nodes.clone(), next_index.clone(), {
                                let mut __v = vec![current.clone()];
                                __v.extend({
                                    let mut __v = vec![oldest.clone()];
                                    __v.extend(tail.clone());
                                    __v
                                });
                                __v
                            })
                        }
                    }
                    Some(older) => {
                        if (((oldest.clone()).level.clone()) == ((current.clone()).level.clone())) {
                            carry(
                                tail.clone(),
                                {
                                    let mut __v = vec![BinderNode {
                                        children: Some(Triple {
                                            first: (oldest.clone()).index.clone(),
                                            second: (older.clone()).index.clone(),
                                            third: (current.clone()).index.clone(),
                                        }),
                                        atom: None::<u64>,
                                        level: ((current.clone()).level.clone()).saturating_add(1),
                                    }];
                                    __v.extend(nodes.clone());
                                    __v
                                },
                                increment_index(next_index.clone()),
                                Root {
                                    index: index_nat(next_index.clone()),
                                    level: ((current.clone()).level.clone()).saturating_add(1),
                                },
                                None::<Root>,
                            )
                        } else {
                            BuildState::State(nodes.clone(), next_index.clone(), {
                                let mut __v = vec![current.clone()];
                                __v.extend({
                                    let mut __v = vec![older.clone()];
                                    __v.extend({
                                        let mut __v = vec![oldest.clone()];
                                        __v.extend(tail.clone());
                                        __v
                                    });
                                    __v
                                });
                                __v
                            })
                        }
                    }
                }
            }
        }
    }

    /// Function `R4.Binder.insertAtom`.
    pub fn insert_atom(atom: u64, state: BuildState) -> BuildState {
        match carry(
            build_frontier(state.clone()),
            {
                let mut __v = vec![BinderNode {
                    children: None::<Triple>,
                    atom: Some(atom.clone()),
                    level: 0u64,
                }];
                __v.extend(build_nodes(state.clone()));
                __v
            },
            increment_index(build_count(state.clone())),
            Root {
                index: index_nat(build_count(state.clone())),
                level: 0u64,
            },
            None::<Root>,
        ) {
            BuildState::State(inserted_nodes, inserted_count, inserted_frontier) => {
                let __scrut = inserted_count.clone();
                if __scrut.is_empty() {
                    BuildState::State(
                        inserted_nodes.clone(),
                        Vec::<bool>::new(),
                        inserted_frontier.clone(),
                    )
                } else {
                    let digit = __scrut[0].clone();
                    let higher = __scrut[1..].to_vec();
                    BuildState::State(
                        inserted_nodes.clone(),
                        {
                            let mut __v = vec![digit.clone()];
                            __v.extend(higher.clone());
                            __v
                        },
                        inserted_frontier.clone(),
                    )
                }
            }
        }
    }

    /// Function `R4.Binder.buildAtoms`.
    pub fn build_atoms(remaining: u64, next_atom: u64, state: BuildState) -> BuildState {
        {
            let __scrut = remaining.clone();
            if __scrut == 0 {
                state.clone()
            } else {
                let prior = __scrut - 1;
                match insert_atom(next_atom.clone(), state.clone()) {
                    BuildState::State(nodes_after, count_after, frontier_after) => build_atoms(
                        prior.clone(),
                        (next_atom.clone()).saturating_add(1),
                        BuildState::State(
                            nodes_after.clone(),
                            count_after.clone(),
                            frontier_after.clone(),
                        ),
                    ),
                }
            }
        }
    }

    /// Function `R4.Binder.reverseNodesLinear`.
    pub fn reverse_nodes_linear(
        items: Vec<BinderNode>,
        result: Vec<BinderNode>,
    ) -> Vec<BinderNode> {
        {
            let __scrut = items.clone();
            if __scrut.is_empty() {
                result.clone()
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                reverse_nodes_linear(tail.clone(), {
                    let mut __v = vec![head.clone()];
                    __v.extend(result.clone());
                    __v
                })
            }
        }
    }

    /// Function `R4.Binder.reverseNodeChunks`.
    pub fn reverse_node_chunks(
        fuel: u64,
        items: Vec<BinderNode>,
        result: Vec<BinderNode>,
    ) -> Vec<BinderNode> {
        {
            let __scrut = fuel.clone();
            if __scrut == 0 {
                reverse_nodes_linear(items.clone(), result.clone())
            } else {
                let fuel_prior = __scrut - 1;
                {
                    let __scrut = items.clone();
                    if __scrut.is_empty() {
                        result.clone()
                    } else {
                        let chunk_head0 = __scrut[0].clone();
                        let chunk_tail0 = __scrut[1..].to_vec();
                        {
                            let __scrut = chunk_tail0.clone();
                            if __scrut.is_empty() {
                                {
                                    let mut __v = vec![chunk_head0.clone()];
                                    __v.extend(result.clone());
                                    __v
                                }
                            } else {
                                let chunk_head1 = __scrut[0].clone();
                                let chunk_tail1 = __scrut[1..].to_vec();
                                {
                                    let __scrut = chunk_tail1.clone();
                                    if __scrut.is_empty() {
                                        {
                                            let mut __v = vec![chunk_head1.clone()];
                                            __v.extend({
                                                let mut __v = vec![chunk_head0.clone()];
                                                __v.extend(result.clone());
                                                __v
                                            });
                                            __v
                                        }
                                    } else {
                                        let chunk_head2 = __scrut[0].clone();
                                        let chunk_tail2 = __scrut[1..].to_vec();
                                        {
                                            let __scrut = chunk_tail2.clone();
                                            if __scrut.is_empty() {
                                                {
                                                    let mut __v = vec![chunk_head2.clone()];
                                                    __v.extend({
                                                        let mut __v = vec![chunk_head1.clone()];
                                                        __v.extend({
                                                            let mut __v = vec![chunk_head0.clone()];
                                                            __v.extend(result.clone());
                                                            __v
                                                        });
                                                        __v
                                                    });
                                                    __v
                                                }
                                            } else {
                                                let chunk_head3 = __scrut[0].clone();
                                                let chunk_tail3 = __scrut[1..].to_vec();
                                                {
                                                    let __scrut = chunk_tail3.clone();
                                                    if __scrut.is_empty() {
                                                        {
                                                            let mut __v = vec![chunk_head3.clone()];
                                                            __v.extend({
                                                                let mut __v =
                                                                    vec![chunk_head2.clone()];
                                                                __v.extend({
                                                                    let mut __v =
                                                                        vec![chunk_head1.clone()];
                                                                    __v.extend({
                                                                        let mut __v = vec![
                                                                            chunk_head0.clone(),
                                                                        ];
                                                                        __v.extend(result.clone());
                                                                        __v
                                                                    });
                                                                    __v
                                                                });
                                                                __v
                                                            });
                                                            __v
                                                        }
                                                    } else {
                                                        let chunk_head4 = __scrut[0].clone();
                                                        let chunk_tail4 = __scrut[1..].to_vec();
                                                        {
                                                            let __scrut = chunk_tail4.clone();
                                                            if __scrut.is_empty() {
                                                                {
                                                                    let mut __v =
                                                                        vec![chunk_head4.clone()];
                                                                    __v.extend({ let mut __v = vec![chunk_head3.clone()]; __v.extend({ let mut __v = vec![chunk_head2.clone()]; __v.extend({ let mut __v = vec![chunk_head1.clone()]; __v.extend({ let mut __v = vec![chunk_head0.clone()]; __v.extend(result.clone()); __v }); __v }); __v }); __v });
                                                                    __v
                                                                }
                                                            } else {
                                                                let chunk_head5 =
                                                                    __scrut[0].clone();
                                                                let chunk_tail5 =
                                                                    __scrut[1..].to_vec();
                                                                {
                                                                    let __scrut =
                                                                        chunk_tail5.clone();
                                                                    if __scrut.is_empty() {
                                                                        {
                                                                            let mut __v =
                                                                                vec![chunk_head5
                                                                                    .clone()];
                                                                            __v.extend({ let mut __v = vec![chunk_head4.clone()]; __v.extend({ let mut __v = vec![chunk_head3.clone()]; __v.extend({ let mut __v = vec![chunk_head2.clone()]; __v.extend({ let mut __v = vec![chunk_head1.clone()]; __v.extend({ let mut __v = vec![chunk_head0.clone()]; __v.extend(result.clone()); __v }); __v }); __v }); __v }); __v });
                                                                            __v
                                                                        }
                                                                    } else {
                                                                        let chunk_head6 =
                                                                            __scrut[0].clone();
                                                                        let chunk_tail6 =
                                                                            __scrut[1..].to_vec();
                                                                        {
                                                                            let __scrut =
                                                                                chunk_tail6.clone();
                                                                            if __scrut.is_empty() {
                                                                                {
                                                                                    let mut __v = vec![
                                                                                        chunk_head6
                                                                                            .clone(
                                                                                            ),
                                                                                    ];
                                                                                    __v.extend({ let mut __v = vec![chunk_head5.clone()]; __v.extend({ let mut __v = vec![chunk_head4.clone()]; __v.extend({ let mut __v = vec![chunk_head3.clone()]; __v.extend({ let mut __v = vec![chunk_head2.clone()]; __v.extend({ let mut __v = vec![chunk_head1.clone()]; __v.extend({ let mut __v = vec![chunk_head0.clone()]; __v.extend(result.clone()); __v }); __v }); __v }); __v }); __v }); __v });
                                                                                    __v
                                                                                }
                                                                            } else {
                                                                                let chunk_head7 =
                                                                                    __scrut[0]
                                                                                        .clone();
                                                                                let chunk_tail7 =
                                                                                    __scrut[1..]
                                                                                        .to_vec();
                                                                                reverse_node_chunks(
                                                                                    fuel_prior
                                                                                        .clone(),
                                                                                    chunk_tail7
                                                                                        .clone(),
                                                                                    {
                                                                                        let mut __v = vec![chunk_head7.clone()];
                                                                                        __v.extend({ let mut __v = vec![chunk_head6.clone()]; __v.extend({ let mut __v = vec![chunk_head5.clone()]; __v.extend({ let mut __v = vec![chunk_head4.clone()]; __v.extend({ let mut __v = vec![chunk_head3.clone()]; __v.extend({ let mut __v = vec![chunk_head2.clone()]; __v.extend({ let mut __v = vec![chunk_head1.clone()]; __v.extend({ let mut __v = vec![chunk_head0.clone()]; __v.extend(result.clone()); __v }); __v }); __v }); __v }); __v }); __v }); __v });
                                                                                        __v
                                                                                    },
                                                                                )
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Function `R4.Binder.reverseNodes`.
    pub fn reverse_nodes(items: Vec<BinderNode>, result: Vec<BinderNode>) -> Vec<BinderNode> {
        reverse_node_chunks(64u64, items.clone(), result.clone())
    }

    /// Function `R4.Binder.maximumRootLevel`.
    pub fn maximum_root_level(items: Vec<Root>, largest: u64) -> u64 {
        {
            let __scrut = items.clone();
            if __scrut.is_empty() {
                largest.clone()
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                maximum_root_level(
                    tail.clone(),
                    if ((largest.clone()) < ((head.clone()).level.clone())) {
                        (head.clone()).level.clone()
                    } else {
                        largest.clone()
                    },
                )
            }
        }
    }

    /// Function `R4.Binder.buildChunks`.
    pub fn build_chunks(
        fuel: u64,
        remaining: u64,
        next_atom: u64,
        state: BuildState,
    ) -> BuildState {
        {
            let __scrut = fuel.clone();
            if __scrut == 0 {
                build_atoms(remaining.clone(), next_atom.clone(), state.clone())
            } else {
                let fuel_prior = __scrut - 1;
                {
                    let __scrut = remaining.clone();
                    if __scrut == 0 {
                        state.clone()
                    } else {
                        let rest0 = __scrut - 1;
                        {
                            let __scrut = rest0.clone();
                            if __scrut == 0 {
                                insert_atom(next_atom.clone(), state.clone())
                            } else {
                                let rest1 = __scrut - 1;
                                {
                                    let __scrut = rest1.clone();
                                    if __scrut == 0 {
                                        match insert_atom(next_atom.clone(), state.clone()) {
                                            BuildState::State(
                                                first_nodes,
                                                first_count,
                                                first_frontier,
                                            ) => insert_atom(
                                                (next_atom.clone()).saturating_add(1),
                                                BuildState::State(
                                                    first_nodes.clone(),
                                                    first_count.clone(),
                                                    first_frontier.clone(),
                                                ),
                                            ),
                                        }
                                    } else {
                                        let rest2 = __scrut - 1;
                                        match insert_atom(next_atom.clone(), state.clone()) {
                                            BuildState::State(
                                                first_nodes,
                                                first_count,
                                                first_frontier,
                                            ) => {
                                                match insert_atom(
                                                    (next_atom.clone()).saturating_add(1),
                                                    BuildState::State(
                                                        first_nodes.clone(),
                                                        first_count.clone(),
                                                        first_frontier.clone(),
                                                    ),
                                                ) {
                                                    BuildState::State(
                                                        second_nodes,
                                                        second_count,
                                                        second_frontier,
                                                    ) => {
                                                        match insert_atom(
                                                            ((next_atom.clone()).saturating_add(1))
                                                                .saturating_add(1),
                                                            BuildState::State(
                                                                second_nodes.clone(),
                                                                second_count.clone(),
                                                                second_frontier.clone(),
                                                            ),
                                                        ) {
                                                            BuildState::State(
                                                                third_nodes,
                                                                third_count,
                                                                third_frontier,
                                                            ) => build_chunks(
                                                                fuel_prior.clone(),
                                                                rest2.clone(),
                                                                (((next_atom.clone())
                                                                    .saturating_add(1))
                                                                .saturating_add(1))
                                                                .saturating_add(1),
                                                                BuildState::State(
                                                                    third_nodes.clone(),
                                                                    third_count.clone(),
                                                                    third_frontier.clone(),
                                                                ),
                                                            ),
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Function `R4.Binder.finishPlan`.
    pub fn finish_plan(context: u64, state: BuildState) -> BinderPlan {
        BinderPlan {
            context: context.clone(),
            nodes: reverse_nodes(build_nodes(state.clone()), Vec::<BinderNode>::new()),
            frontier: reverse_roots(build_frontier(state.clone()), Vec::<u64>::new()),
            stages: (maximum_root_level(build_frontier(state.clone()), 0u64)).saturating_add(1),
        }
    }

    /// Function `R4.Binder.construct`.
    pub fn construct(context: u64) -> Construction {
        if valid_context(context.clone()) {
            Construction::Built(finish_plan(
                context.clone(),
                build_chunks(
                    context.clone(),
                    context.clone(),
                    0u64,
                    BuildState::State(
                        Vec::<BinderNode>::new(),
                        Vec::<bool>::new(),
                        Vec::<Root>::new(),
                    ),
                ),
            ))
        } else {
            Construction::InvalidContext
        }
    }

    /// Function `R4.Binder.lookupNode`.
    pub fn lookup_node(items: Vec<BinderNode>, index: u64) -> Option<BinderNode> {
        {
            let __scrut = items.clone();
            if __scrut.is_empty() {
                None::<BinderNode>
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                {
                    let __scrut = index.clone();
                    if __scrut == 0 {
                        Some(head.clone())
                    } else {
                        let prior = __scrut - 1;
                        lookup_node(tail.clone(), prior.clone())
                    }
                }
            }
        }
    }

    /// Function `R4.Binder.checkChild`.
    pub fn check_child(
        index: u64,
        parent_level: u64,
        child: u64,
        used: Vec<u64>,
        nodes: Vec<BinderNode>,
    ) -> Option<Vec<u64>> {
        if (((child.clone()) < (index.clone())) && (!(contains_nat(used.clone(), child.clone())))) {
            match lookup_node(nodes.clone(), child.clone()) {
                None => None::<Vec<u64>>,
                Some(child_node) => {
                    if ((((child_node.clone()).level.clone()).saturating_add(1))
                        == (parent_level.clone()))
                    {
                        Some({
                            let mut __v = vec![child.clone()];
                            __v.extend(used.clone());
                            __v
                        })
                    } else {
                        None::<Vec<u64>>
                    }
                }
            }
        } else {
            None::<Vec<u64>>
        }
    }

    /// Function `R4.Binder.checkTriple`.
    pub fn check_triple(
        index: u64,
        parent_level: u64,
        children: Triple,
        used: Vec<u64>,
        nodes: Vec<BinderNode>,
    ) -> Option<Vec<u64>> {
        match check_child(
            index.clone(),
            parent_level.clone(),
            (children.clone()).first.clone(),
            used.clone(),
            nodes.clone(),
        ) {
            None => None::<Vec<u64>>,
            Some(first_used) => {
                match check_child(
                    index.clone(),
                    parent_level.clone(),
                    (children.clone()).second.clone(),
                    first_used.clone(),
                    nodes.clone(),
                ) {
                    None => None::<Vec<u64>>,
                    Some(second_used) => check_child(
                        index.clone(),
                        parent_level.clone(),
                        (children.clone()).third.clone(),
                        second_used.clone(),
                        nodes.clone(),
                    ),
                }
            }
        }
    }

    /// Function `R4.Binder.scanHead`.
    pub fn scan_head(
        node: BinderNode,
        index: u64,
        state: ScanState,
        nodes: Vec<BinderNode>,
    ) -> ScanResult {
        match (node.clone()).children.clone() {
            None => match (node.clone()).atom.clone() {
                None => ScanResult::Failed(BinderError::NodeShape),
                Some(atom) => {
                    if ((((node.clone()).level.clone()) == (0u64))
                        && ((atom.clone()) == ((state.clone()).next_atom.clone())))
                    {
                        ScanResult::Scanned(ScanState {
                            next_atom: ((state.clone()).next_atom.clone()).saturating_add(1),
                            maximum_level: (state.clone()).maximum_level.clone(),
                            used: (state.clone()).used.clone(),
                        })
                    } else {
                        ScanResult::Failed(BinderError::NodeShape)
                    }
                }
            },
            Some(children) => match (node.clone()).atom.clone() {
                None => {
                    if ((0u64) < ((node.clone()).level.clone())) {
                        match check_triple(
                            index.clone(),
                            (node.clone()).level.clone(),
                            children.clone(),
                            (state.clone()).used.clone(),
                            nodes.clone(),
                        ) {
                            None => ScanResult::Failed(BinderError::ChildLink),
                            Some(used_after) => ScanResult::Scanned(ScanState {
                                next_atom: (state.clone()).next_atom.clone(),
                                maximum_level: if (((state.clone()).maximum_level.clone())
                                    < ((node.clone()).level.clone()))
                                {
                                    (node.clone()).level.clone()
                                } else {
                                    (state.clone()).maximum_level.clone()
                                },
                                used: used_after.clone(),
                            }),
                        }
                    } else {
                        ScanResult::Failed(BinderError::NodeShape)
                    }
                }
                Some(atom) => ScanResult::Failed(BinderError::NodeShape),
            },
        }
    }

    /// Function `R4.Binder.scanNodes`.
    pub fn scan_nodes(
        items: Vec<BinderNode>,
        index: u64,
        state: ScanState,
        nodes: Vec<BinderNode>,
    ) -> ScanResult {
        {
            let __scrut = items.clone();
            if __scrut.is_empty() {
                ScanResult::Scanned(state.clone())
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                match scan_head(head.clone(), index.clone(), state.clone(), nodes.clone()) {
                    ScanResult::Failed(error) => ScanResult::Failed(error.clone()),
                    ScanResult::Scanned(next_state) => scan_nodes(
                        tail.clone(),
                        (index.clone()).saturating_add(1),
                        next_state.clone(),
                        nodes.clone(),
                    ),
                }
            }
        }
    }

    /// Function `R4.Binder.checkRoots`.
    pub fn check_roots(
        frontier: Vec<u64>,
        size: u64,
        used: Vec<u64>,
        roots: Vec<u64>,
    ) -> Option<Vec<u64>> {
        {
            let __scrut = frontier.clone();
            if __scrut.is_empty() {
                Some(roots.clone())
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                if (((head.clone()) < (size.clone()))
                    && ((!(contains_nat(roots.clone(), head.clone())))
                        && (!(contains_nat(used.clone(), head.clone())))))
                {
                    check_roots(tail.clone(), size.clone(), used.clone(), {
                        let mut __v = vec![head.clone()];
                        __v.extend(roots.clone());
                        __v
                    })
                } else {
                    None::<Vec<u64>>
                }
            }
        }
    }

    /// Function `R4.Binder.completeRoots`.
    pub fn complete_roots(
        items: Vec<BinderNode>,
        index: u64,
        used: Vec<u64>,
        roots: Vec<u64>,
    ) -> bool {
        {
            let __scrut = items.clone();
            if __scrut.is_empty() {
                true
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                ((if contains_nat(roots.clone(), index.clone()) {
                    (!(contains_nat(used.clone(), index.clone())))
                } else {
                    contains_nat(used.clone(), index.clone())
                }) && (complete_roots(
                    tail.clone(),
                    (index.clone()).saturating_add(1),
                    used.clone(),
                    roots.clone(),
                )))
            }
        }
    }

    /// Function `R4.Binder.traverse`.
    pub fn traverse(
        fuel: u64,
        stack: Vec<u64>,
        expected: u64,
        context: u64,
        nodes: Vec<BinderNode>,
    ) -> Validation {
        {
            let __scrut = fuel.clone();
            if __scrut == 0 {
                {
                    let __scrut = stack.clone();
                    if __scrut.is_empty() {
                        if ((expected.clone()) == (context.clone())) {
                            Validation::Valid
                        } else {
                            Validation::Invalid(BinderError::AtomCoverage)
                        }
                    } else {
                        let head = __scrut[0].clone();
                        let tail = __scrut[1..].to_vec();
                        Validation::Invalid(BinderError::AtomCoverage)
                    }
                }
            } else {
                let prior = __scrut - 1;
                {
                    let __scrut = stack.clone();
                    if __scrut.is_empty() {
                        if ((expected.clone()) == (context.clone())) {
                            Validation::Valid
                        } else {
                            Validation::Invalid(BinderError::AtomCoverage)
                        }
                    } else {
                        let head = __scrut[0].clone();
                        let tail = __scrut[1..].to_vec();
                        match lookup_node(nodes.clone(), head.clone()) {
                            None => Validation::Invalid(BinderError::AtomOrder),
                            Some(node) => match (node.clone()).children.clone() {
                                None => match (node.clone()).atom.clone() {
                                    None => Validation::Invalid(BinderError::AtomOrder),
                                    Some(atom) => {
                                        if ((atom.clone()) == (expected.clone())) {
                                            traverse(
                                                prior.clone(),
                                                tail.clone(),
                                                (expected.clone()).saturating_add(1),
                                                context.clone(),
                                                nodes.clone(),
                                            )
                                        } else {
                                            Validation::Invalid(BinderError::AtomOrder)
                                        }
                                    }
                                },
                                Some(children) => match (node.clone()).atom.clone() {
                                    None => traverse(
                                        prior.clone(),
                                        {
                                            let mut __v = vec![(children.clone()).first.clone()];
                                            __v.extend({
                                                let mut __v =
                                                    vec![(children.clone()).second.clone()];
                                                __v.extend({
                                                    let mut __v =
                                                        vec![(children.clone()).third.clone()];
                                                    __v.extend(tail.clone());
                                                    __v
                                                });
                                                __v
                                            });
                                            __v
                                        },
                                        expected.clone(),
                                        context.clone(),
                                        nodes.clone(),
                                    ),
                                    Some(atom) => Validation::Invalid(BinderError::AtomOrder),
                                },
                            },
                        }
                    }
                }
            }
        }
    }

    /// Function `R4.Binder.finishValidation`.
    pub fn finish_validation(plan: BinderPlan, state: ScanState) -> Validation {
        if ((((state.clone()).next_atom.clone()) == ((plan.clone()).context.clone()))
            && (((plan.clone()).stages.clone())
                == (((state.clone()).maximum_level.clone()).saturating_add(1))))
        {
            match check_roots(
                (plan.clone()).frontier.clone(),
                length_node((plan.clone()).nodes.clone()),
                (state.clone()).used.clone(),
                Vec::<u64>::new(),
            ) {
                None => Validation::Invalid(BinderError::FrontierRoot),
                Some(roots) => {
                    if complete_roots(
                        (plan.clone()).nodes.clone(),
                        0u64,
                        (state.clone()).used.clone(),
                        roots.clone(),
                    ) {
                        traverse(
                            length_node((plan.clone()).nodes.clone()),
                            (plan.clone()).frontier.clone(),
                            0u64,
                            (plan.clone()).context.clone(),
                            (plan.clone()).nodes.clone(),
                        )
                    } else {
                        Validation::Invalid(BinderError::OmittedRoot)
                    }
                }
            }
        } else {
            Validation::Invalid(BinderError::ContextStages)
        }
    }

    /// Function `R4.Binder.validate`.
    pub fn validate(plan: BinderPlan) -> Validation {
        if ((valid_context((plan.clone()).context.clone()))
            && (((0u64) < (length_node((plan.clone()).nodes.clone())))
                && (((0u64) < (length_nat((plan.clone()).frontier.clone())))
                    && ((length_nat((plan.clone()).frontier.clone())) <= (16u64)))))
        {
            match scan_nodes(
                (plan.clone()).nodes.clone(),
                0u64,
                ScanState {
                    next_atom: 0u64,
                    maximum_level: 0u64,
                    used: Vec::<u64>::new(),
                },
                (plan.clone()).nodes.clone(),
            ) {
                ScanResult::Failed(error) => Validation::Invalid(error.clone()),
                ScanResult::Scanned(state) => finish_validation(plan.clone(), state.clone()),
            }
        } else {
            Validation::Invalid(BinderError::Inventory)
        }
    }

    /// Function `R4.Binder.oneNodeWords`.
    pub fn one_node_words(node: BinderNode) -> Vec<u64> {
        match (node.clone()).children.clone() {
            None => {
                let mut __v = vec![4294967295u64];
                __v.extend({
                    let mut __v = vec![4294967295u64];
                    __v.extend({
                        let mut __v = vec![4294967295u64];
                        __v.extend({
                            let mut __v = vec![match (node.clone()).atom.clone() {
                                None => 4294967295u64,
                                Some(atom) => atom.clone(),
                            }];
                            __v.extend(Vec::<u64>::new());
                            __v
                        });
                        __v
                    });
                    __v
                });
                __v
            }
            Some(children) => {
                let mut __v = vec![(children.clone()).first.clone()];
                __v.extend({
                    let mut __v = vec![(children.clone()).second.clone()];
                    __v.extend({
                        let mut __v = vec![(children.clone()).third.clone()];
                        __v.extend({
                            let mut __v = vec![match (node.clone()).atom.clone() {
                                None => 4294967295u64,
                                Some(atom) => atom.clone(),
                            }];
                            __v.extend(Vec::<u64>::new());
                            __v
                        });
                        __v
                    });
                    __v
                });
                __v
            }
        }
    }

    /// Function `R4.Binder.nodeWords`.
    pub fn node_words(nodes: Vec<BinderNode>) -> Vec<u64> {
        {
            let __scrut = nodes.clone();
            if __scrut.is_empty() {
                Vec::<u64>::new()
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                append_nat(one_node_words(head.clone()), node_words(tail.clone()))
            }
        }
    }
}

/// Generated from `R4.IntegerStack` (`src/R4/IntegerStack.lex.tex`).
pub mod r4_integer_stack {
    /// Inductive type `R4.IntegerStack.HammingOutcome`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum HammingOutcome {
        /// Variant `value`.
        Value(u64),
        /// Variant `outOfMargin`.
        OutOfMargin,
        /// Variant `zeroMargin`.
        ZeroMargin,
        /// Variant `invalidWindow`.
        InvalidWindow,
    }

    /// Inductive type `R4.IntegerStack.MemoryKeyKind`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum MemoryKeyKind {
        /// Variant `opening`.
        Opening(u64),
        /// Variant `reply`.
        Reply(u64, u64, u64),
        /// Variant `inactive`.
        Inactive,
    }

    /// Structure `R4.IntegerStack.MemoryEntry`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct MemoryEntry {
        /// Field `anchorToken`.
        pub anchor_token: u64,
        /// Field `replyLength`.
        pub reply_length: u64,
        /// Field `currentToken`.
        pub current_token: u64,
        /// Field `isReply`.
        pub is_reply: bool,
        /// Field `storedTarget`.
        pub stored_target: u64,
    }

    /// Inductive type `R4.IntegerStack.TurnMemoryOutcome`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum TurnMemoryOutcome {
        /// Variant `openingHit`.
        OpeningHit(u64),
        /// Variant `replyHit`.
        ReplyHit(u64),
        /// Variant `openingMiss`.
        OpeningMiss(u64),
        /// Variant `replyMiss`.
        ReplyMiss,
        /// Variant `inactive`.
        Inactive,
        /// Variant `invalidReplyStart`.
        InvalidReplyStart,
        /// Variant `snapshotRefusedTurnMemory`.
        SnapshotRefusedTurnMemory,
    }

    /// Structure `R4.IntegerStack.PointRow`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct PointRow {
        /// Field `c0`.
        pub c0: u64,
        /// Field `c1`.
        pub c1: u64,
        /// Field `c2`.
        pub c2: u64,
        /// Field `c3`.
        pub c3: u64,
        /// Field `targetToken`.
        pub target_token: u64,
    }

    /// Inductive type `R4.IntegerStack.PointKeyOutcome`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum PointKeyOutcome {
        /// Variant `value`.
        Value(u64, u64),
        /// Variant `emptyTable`.
        EmptyTable,
        /// Variant `invalidDimension`.
        InvalidDimension,
    }

    /// Function `R4.IntegerStack.predecessor`.
    pub fn predecessor(value: u64) -> u64 {
        {
            let __scrut = value.clone();
            if __scrut == 0 {
                0u64
            } else {
                let prior = __scrut - 1;
                prior.clone()
            }
        }
    }

    /// Function `R4.IntegerStack.subtractNat`.
    pub fn subtract_nat(left: u64, right: u64) -> u64 {
        {
            let __scrut = right.clone();
            if __scrut == 0 {
                left.clone()
            } else {
                let prior = __scrut - 1;
                subtract_nat(predecessor(left.clone()), prior.clone())
            }
        }
    }

    /// Function `R4.IntegerStack.scaleAccumulator`.
    pub fn scale_accumulator(value: u64, times: u64, total: u64) -> u64 {
        {
            let __scrut = times.clone();
            if __scrut == 0 {
                total.clone()
            } else {
                let prior = __scrut - 1;
                scale_accumulator(
                    value.clone(),
                    prior.clone(),
                    (total.clone()).saturating_add(value.clone()),
                )
            }
        }
    }

    /// Function `R4.IntegerStack.scaleNat`.
    pub fn scale_nat(value: u64, times: u64) -> u64 {
        scale_accumulator(value.clone(), times.clone(), 0u64)
    }

    /// Function `R4.IntegerStack.absDiff`.
    pub fn abs_diff(a: u64, b: u64) -> u64 {
        (subtract_nat(a.clone(), b.clone())).saturating_add(subtract_nat(b.clone(), a.clone()))
    }

    /// Function `R4.IntegerStack.hammingMatchWeight`.
    pub fn hamming_match_weight(distance: u64, margin: u64, scale: u64) -> HammingOutcome {
        if ((margin.clone()) == (0u64)) {
            HammingOutcome::ZeroMargin
        } else {
            if ((margin.clone()) <= (distance.clone())) {
                HammingOutcome::OutOfMargin
            } else {
                HammingOutcome::Value(scale_nat(
                    subtract_nat(margin.clone(), distance.clone()),
                    scale.clone(),
                ))
            }
        }
    }

    /// Function `R4.IntegerStack.inductionCandidateValid`.
    pub fn induction_candidate_valid(
        window_start: u64,
        current_pos: u64,
        candidate_pos: u64,
    ) -> bool {
        (((1u64) <= (candidate_pos.clone()))
            && (((window_start.clone()) <= (candidate_pos.clone()))
                && ((candidate_pos.clone()) < (current_pos.clone()))))
    }

    /// Function `R4.IntegerStack.evalInductionHead`.
    pub fn eval_induction_head(
        window_start: u64,
        current_pos: u64,
        candidate_pos: u64,
        prev_token: u64,
        query_token: u64,
        distance: u64,
        margin: u64,
        scale: u64,
    ) -> HammingOutcome {
        if induction_candidate_valid(
            window_start.clone(),
            current_pos.clone(),
            candidate_pos.clone(),
        ) {
            if ((prev_token.clone()) == (query_token.clone())) {
                hamming_match_weight(distance.clone(), margin.clone(), scale.clone())
            } else {
                HammingOutcome::OutOfMargin
            }
        } else {
            HammingOutcome::InvalidWindow
        }
    }

    /// Function `R4.IntegerStack.classifyTurnKey`.
    pub fn classify_turn_key(
        row: u64,
        reply_start: u64,
        anchor_token: u64,
        current_token: u64,
    ) -> MemoryKeyKind {
        if ((reply_start.clone()) == (0u64)) {
            MemoryKeyKind::Inactive
        } else {
            if (((row.clone()).saturating_add(1u64)) < (reply_start.clone())) {
                MemoryKeyKind::Inactive
            } else {
                if (((row.clone()).saturating_add(1u64)) == (reply_start.clone())) {
                    MemoryKeyKind::Opening(anchor_token.clone())
                } else {
                    MemoryKeyKind::Reply(
                        anchor_token.clone(),
                        subtract_nat((row.clone()).saturating_add(1u64), reply_start.clone()),
                        current_token.clone(),
                    )
                }
            }
        }
    }

    /// Function `R4.IntegerStack.classifyTurnStep`.
    pub fn classify_turn_step(
        row: u64,
        reply_start: u64,
        anchor_token: u64,
        current_token: u64,
    ) -> TurnMemoryOutcome {
        if ((reply_start.clone()) == (0u64)) {
            TurnMemoryOutcome::InvalidReplyStart
        } else {
            if (((row.clone()).saturating_add(1u64)) < (reply_start.clone())) {
                TurnMemoryOutcome::Inactive
            } else {
                if (((row.clone()).saturating_add(1u64)) == (reply_start.clone())) {
                    TurnMemoryOutcome::OpeningHit(anchor_token.clone())
                } else {
                    TurnMemoryOutcome::ReplyHit(subtract_nat(
                        (row.clone()).saturating_add(1u64),
                        reply_start.clone(),
                    ))
                }
            }
        }
    }

    /// Function `R4.IntegerStack.lookupOpeningTable`.
    pub fn lookup_opening_table(entries: Vec<MemoryEntry>, anchor_token: u64) -> TurnMemoryOutcome {
        {
            let __scrut = entries.clone();
            if __scrut.is_empty() {
                TurnMemoryOutcome::OpeningMiss(anchor_token.clone())
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                if ((!((head.clone()).is_reply.clone()))
                    && (((head.clone()).anchor_token.clone()) == (anchor_token.clone())))
                {
                    TurnMemoryOutcome::OpeningHit((head.clone()).stored_target.clone())
                } else {
                    lookup_opening_table(tail.clone(), anchor_token.clone())
                }
            }
        }
    }

    /// Function `R4.IntegerStack.lookupReplyTable`.
    pub fn lookup_reply_table(
        entries: Vec<MemoryEntry>,
        anchor_token: u64,
        reply_length: u64,
        current_token: u64,
    ) -> TurnMemoryOutcome {
        {
            let __scrut = entries.clone();
            if __scrut.is_empty() {
                TurnMemoryOutcome::ReplyMiss
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                if (((head.clone()).is_reply.clone())
                    && ((((head.clone()).anchor_token.clone()) == (anchor_token.clone()))
                        && ((((head.clone()).reply_length.clone()) == (reply_length.clone()))
                            && (((head.clone()).current_token.clone())
                                == (current_token.clone())))))
                {
                    TurnMemoryOutcome::ReplyHit((head.clone()).stored_target.clone())
                } else {
                    lookup_reply_table(
                        tail.clone(),
                        anchor_token.clone(),
                        reply_length.clone(),
                        current_token.clone(),
                    )
                }
            }
        }
    }

    /// Function `R4.IntegerStack.evalTurnMemory`.
    pub fn eval_turn_memory(
        entries: Vec<MemoryEntry>,
        row: u64,
        reply_start: u64,
        anchor_token: u64,
        current_token: u64,
    ) -> TurnMemoryOutcome {
        if ((reply_start.clone()) == (0u64)) {
            TurnMemoryOutcome::InvalidReplyStart
        } else {
            if (((row.clone()).saturating_add(1u64)) < (reply_start.clone())) {
                TurnMemoryOutcome::Inactive
            } else {
                if (((row.clone()).saturating_add(1u64)) == (reply_start.clone())) {
                    lookup_opening_table(entries.clone(), anchor_token.clone())
                } else {
                    lookup_reply_table(
                        entries.clone(),
                        anchor_token.clone(),
                        subtract_nat((row.clone()).saturating_add(1u64), reply_start.clone()),
                        current_token.clone(),
                    )
                }
            }
        }
    }

    /// Function `R4.IntegerStack.interpolateReadout`.
    pub fn interpolate_readout(
        point_coord: u64,
        target_coord: u64,
        weight: u64,
        scale_max: u64,
    ) -> u64 {
        (scale_nat(
            point_coord.clone(),
            subtract_nat(scale_max.clone(), weight.clone()),
        ))
        .saturating_add(scale_nat(target_coord.clone(), weight.clone()))
    }

    /// Function `R4.IntegerStack.guardSessionSnapshot`.
    pub fn guard_session_snapshot(has_turn_memory: bool, session_id: u64) -> TurnMemoryOutcome {
        if has_turn_memory.clone() {
            TurnMemoryOutcome::SnapshotRefusedTurnMemory
        } else {
            TurnMemoryOutcome::OpeningHit(session_id.clone())
        }
    }

    /// Function `R4.IntegerStack.rowL1Distance`.
    pub fn row_l1_distance(q0: u64, q1: u64, q2: u64, q3: u64, row: PointRow) -> u64 {
        ((abs_diff(q0.clone(), (row.clone()).c0.clone()))
            .saturating_add(abs_diff(q1.clone(), (row.clone()).c1.clone())))
        .saturating_add(
            (abs_diff(q2.clone(), (row.clone()).c2.clone()))
                .saturating_add(abs_diff(q3.clone(), (row.clone()).c3.clone())),
        )
    }

    /// Function `R4.IntegerStack.minDistanceTail`.
    pub fn min_distance_tail(
        q0: u64,
        q1: u64,
        q2: u64,
        q3: u64,
        best: u64,
        rows: Vec<PointRow>,
    ) -> u64 {
        {
            let __scrut = rows.clone();
            if __scrut.is_empty() {
                best.clone()
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                {
                    let dist: u64 = row_l1_distance(
                        q0.clone(),
                        q1.clone(),
                        q2.clone(),
                        q3.clone(),
                        head.clone(),
                    );
                    if ((dist.clone()) < (best.clone())) {
                        min_distance_tail(
                            q0.clone(),
                            q1.clone(),
                            q2.clone(),
                            q3.clone(),
                            dist.clone(),
                            tail.clone(),
                        )
                    } else {
                        min_distance_tail(
                            q0.clone(),
                            q1.clone(),
                            q2.clone(),
                            q3.clone(),
                            best.clone(),
                            tail.clone(),
                        )
                    }
                }
            }
        }
    }

    /// Function `R4.IntegerStack.minTableDistance`.
    pub fn min_table_distance(q0: u64, q1: u64, q2: u64, q3: u64, rows: Vec<PointRow>) -> u64 {
        {
            let __scrut = rows.clone();
            if __scrut.is_empty() {
                0u64
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                min_distance_tail(
                    q0.clone(),
                    q1.clone(),
                    q2.clone(),
                    q3.clone(),
                    row_l1_distance(q0.clone(), q1.clone(), q2.clone(), q3.clone(), head.clone()),
                    tail.clone(),
                )
            }
        }
    }

    /// Function `R4.IntegerStack.countTargetAtDistance`.
    pub fn count_target_at_distance(
        q0: u64,
        q1: u64,
        q2: u64,
        q3: u64,
        target: u64,
        dist: u64,
        rows: Vec<PointRow>,
    ) -> u64 {
        {
            let __scrut = rows.clone();
            if __scrut.is_empty() {
                0u64
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                (if ((((head.clone()).target_token.clone()) == (target.clone()))
                    && ((row_l1_distance(
                        q0.clone(),
                        q1.clone(),
                        q2.clone(),
                        q3.clone(),
                        head.clone(),
                    )) == (dist.clone())))
                {
                    1u64
                } else {
                    0u64
                })
                .saturating_add(count_target_at_distance(
                    q0.clone(),
                    q1.clone(),
                    q2.clone(),
                    q3.clone(),
                    target.clone(),
                    dist.clone(),
                    tail.clone(),
                ))
            }
        }
    }

    /// Function `R4.IntegerStack.pluralityCandidateBeats`.
    pub fn plurality_candidate_beats(
        cand_token: u64,
        cand_count: u64,
        best_token: u64,
        best_count: u64,
    ) -> bool {
        (((best_count.clone()) < (cand_count.clone()))
            || (((cand_count.clone()) == (best_count.clone()))
                && ((cand_token.clone()) < (best_token.clone()))))
    }

    /// Function `R4.IntegerStack.selectPluralityBest`.
    pub fn select_plurality_best(
        q0: u64,
        q1: u64,
        q2: u64,
        q3: u64,
        min_dist: u64,
        best_token: u64,
        best_count: u64,
        remaining: Vec<PointRow>,
        all_rows: Vec<PointRow>,
    ) -> u64 {
        {
            let __scrut = remaining.clone();
            if __scrut.is_empty() {
                best_token.clone()
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                if ((row_l1_distance(q0.clone(), q1.clone(), q2.clone(), q3.clone(), head.clone()))
                    == (min_dist.clone()))
                {
                    {
                        let cnt: u64 = count_target_at_distance(
                            q0.clone(),
                            q1.clone(),
                            q2.clone(),
                            q3.clone(),
                            (head.clone()).target_token.clone(),
                            min_dist.clone(),
                            all_rows.clone(),
                        );
                        if plurality_candidate_beats(
                            (head.clone()).target_token.clone(),
                            cnt.clone(),
                            best_token.clone(),
                            best_count.clone(),
                        ) {
                            select_plurality_best(
                                q0.clone(),
                                q1.clone(),
                                q2.clone(),
                                q3.clone(),
                                min_dist.clone(),
                                (head.clone()).target_token.clone(),
                                cnt.clone(),
                                tail.clone(),
                                all_rows.clone(),
                            )
                        } else {
                            select_plurality_best(
                                q0.clone(),
                                q1.clone(),
                                q2.clone(),
                                q3.clone(),
                                min_dist.clone(),
                                best_token.clone(),
                                best_count.clone(),
                                tail.clone(),
                                all_rows.clone(),
                            )
                        }
                    }
                } else {
                    select_plurality_best(
                        q0.clone(),
                        q1.clone(),
                        q2.clone(),
                        q3.clone(),
                        min_dist.clone(),
                        best_token.clone(),
                        best_count.clone(),
                        tail.clone(),
                        all_rows.clone(),
                    )
                }
            }
        }
    }

    /// Function `R4.IntegerStack.lookupPointKeyPlurality`.
    pub fn lookup_point_key_plurality(
        q0: u64,
        q1: u64,
        q2: u64,
        q3: u64,
        rows: Vec<PointRow>,
    ) -> PointKeyOutcome {
        {
            let __scrut = rows.clone();
            if __scrut.is_empty() {
                PointKeyOutcome::EmptyTable
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                {
                    let all_rows: Vec<PointRow> = {
                        let mut __v = vec![head.clone()];
                        __v.extend(tail.clone());
                        __v
                    };
                    {
                        let min_dist: u64 = min_table_distance(
                            q0.clone(),
                            q1.clone(),
                            q2.clone(),
                            q3.clone(),
                            all_rows.clone(),
                        );
                        PointKeyOutcome::Value(
                            select_plurality_best(
                                q0.clone(),
                                q1.clone(),
                                q2.clone(),
                                q3.clone(),
                                min_dist.clone(),
                                0u64,
                                0u64,
                                all_rows.clone(),
                                all_rows.clone(),
                            ),
                            min_dist.clone(),
                        )
                    }
                }
            }
        }
    }

    /// Function `R4.IntegerStack.resolveTurnWithPointKey`.
    pub fn resolve_turn_with_point_key(
        entries: Vec<MemoryEntry>,
        points: Vec<PointRow>,
        row: u64,
        reply_start: u64,
        anchor_token: u64,
        current_token: u64,
        q0: u64,
        q1: u64,
        q2: u64,
        q3: u64,
    ) -> TurnMemoryOutcome {
        match eval_turn_memory(
            entries.clone(),
            row.clone(),
            reply_start.clone(),
            anchor_token.clone(),
            current_token.clone(),
        ) {
            TurnMemoryOutcome::OpeningHit(tok) => TurnMemoryOutcome::OpeningHit(tok.clone()),
            TurnMemoryOutcome::ReplyHit(tok) => TurnMemoryOutcome::ReplyHit(tok.clone()),
            TurnMemoryOutcome::OpeningMiss(missed_anchor) => {
                match lookup_point_key_plurality(
                    q0.clone(),
                    q1.clone(),
                    q2.clone(),
                    q3.clone(),
                    points.clone(),
                ) {
                    PointKeyOutcome::Value(selected_token, min_dist) => {
                        TurnMemoryOutcome::OpeningHit(selected_token.clone())
                    }
                    PointKeyOutcome::EmptyTable => {
                        TurnMemoryOutcome::OpeningMiss(missed_anchor.clone())
                    }
                    PointKeyOutcome::InvalidDimension => {
                        TurnMemoryOutcome::OpeningMiss(missed_anchor.clone())
                    }
                }
            }
            TurnMemoryOutcome::ReplyMiss => TurnMemoryOutcome::ReplyMiss,
            TurnMemoryOutcome::Inactive => TurnMemoryOutcome::Inactive,
            TurnMemoryOutcome::InvalidReplyStart => TurnMemoryOutcome::InvalidReplyStart,
            TurnMemoryOutcome::SnapshotRefusedTurnMemory => {
                TurnMemoryOutcome::SnapshotRefusedTurnMemory
            }
        }
    }

    /// Function `R4.IntegerStack.retainLambdaQ31`.
    pub fn retain_lambda_q31(head_index: u64) -> u64 {
        if ((head_index.clone()) == (0u64)) {
            1073741824u64
        } else {
            if ((head_index.clone()) == (1u64)) {
                1610612736u64
            } else {
                if ((head_index.clone()) == (2u64)) {
                    1879048192u64
                } else {
                    if ((head_index.clone()) == (3u64)) {
                        2013265920u64
                    } else {
                        if ((head_index.clone()) == (4u64)) {
                            2080374784u64
                        } else {
                            if ((head_index.clone()) == (5u64)) {
                                2113929216u64
                            } else {
                                if ((head_index.clone()) == (6u64)) {
                                    2130706432u64
                                } else {
                                    if ((head_index.clone()) == (7u64)) {
                                        2139095040u64
                                    } else {
                                        if ((head_index.clone()) == (8u64)) {
                                            2143289344u64
                                        } else {
                                            if ((head_index.clone()) == (9u64)) {
                                                2145386496u64
                                            } else {
                                                if ((head_index.clone()) == (10u64)) {
                                                    2146435072u64
                                                } else {
                                                    if ((head_index.clone()) == (11u64)) {
                                                        2146959360u64
                                                    } else {
                                                        if ((head_index.clone()) == (12u64)) {
                                                            2147221504u64
                                                        } else {
                                                            if ((head_index.clone()) == (13u64)) {
                                                                2147352576u64
                                                            } else {
                                                                if ((head_index.clone()) == (14u64))
                                                                {
                                                                    2147418112u64
                                                                } else {
                                                                    if ((head_index.clone())
                                                                        == (15u64))
                                                                    {
                                                                        2147450880u64
                                                                    } else {
                                                                        0u64
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Function `R4.IntegerStack.nearestRowTail`.
    pub fn nearest_row_tail(
        q0: u64,
        q1: u64,
        q2: u64,
        q3: u64,
        best_token: u64,
        best_dist: u64,
        rows: Vec<PointRow>,
    ) -> PointKeyOutcome {
        {
            let __scrut = rows.clone();
            if __scrut.is_empty() {
                PointKeyOutcome::Value(best_token.clone(), best_dist.clone())
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                {
                    let dist: u64 = row_l1_distance(
                        q0.clone(),
                        q1.clone(),
                        q2.clone(),
                        q3.clone(),
                        head.clone(),
                    );
                    if (((dist.clone()) < (best_dist.clone()))
                        || (((dist.clone()) == (best_dist.clone()))
                            && (((head.clone()).target_token.clone()) < (best_token.clone()))))
                    {
                        nearest_row_tail(
                            q0.clone(),
                            q1.clone(),
                            q2.clone(),
                            q3.clone(),
                            (head.clone()).target_token.clone(),
                            dist.clone(),
                            tail.clone(),
                        )
                    } else {
                        nearest_row_tail(
                            q0.clone(),
                            q1.clone(),
                            q2.clone(),
                            q3.clone(),
                            best_token.clone(),
                            best_dist.clone(),
                            tail.clone(),
                        )
                    }
                }
            }
        }
    }

    /// Function `R4.IntegerStack.wholeTableNearest`.
    pub fn whole_table_nearest(
        q0: u64,
        q1: u64,
        q2: u64,
        q3: u64,
        writer_rows: Vec<PointRow>,
    ) -> PointKeyOutcome {
        {
            let __scrut = writer_rows.clone();
            if __scrut.is_empty() {
                PointKeyOutcome::EmptyTable
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                nearest_row_tail(
                    q0.clone(),
                    q1.clone(),
                    q2.clone(),
                    q3.clone(),
                    (head.clone()).target_token.clone(),
                    row_l1_distance(q0.clone(), q1.clone(), q2.clone(), q3.clone(), head.clone()),
                    tail.clone(),
                )
            }
        }
    }

    /// Function `R4.IntegerStack.generateProseTokens`.
    pub fn generate_prose_tokens(
        entries: Vec<MemoryEntry>,
        anchor_token: u64,
        reply_length: u64,
        current_token: u64,
        max_steps: u64,
    ) -> Vec<u64> {
        {
            let __scrut = max_steps.clone();
            if __scrut == 0 {
                Vec::<u64>::new()
            } else {
                let remaining_steps = __scrut - 1;
                match lookup_reply_table(
                    entries.clone(),
                    anchor_token.clone(),
                    reply_length.clone(),
                    current_token.clone(),
                ) {
                    TurnMemoryOutcome::OpeningHit(unused) => Vec::<u64>::new(),
                    TurnMemoryOutcome::ReplyHit(next_token) => {
                        if ((next_token.clone()) == (1u64)) {
                            Vec::<u64>::new()
                        } else {
                            {
                                let mut __v = vec![next_token.clone()];
                                __v.extend(generate_prose_tokens(
                                    entries.clone(),
                                    anchor_token.clone(),
                                    (reply_length.clone()).saturating_add(1u64),
                                    next_token.clone(),
                                    remaining_steps.clone(),
                                ));
                                __v
                            }
                        }
                    }
                    TurnMemoryOutcome::OpeningMiss(unused) => Vec::<u64>::new(),
                    TurnMemoryOutcome::ReplyMiss => Vec::<u64>::new(),
                    TurnMemoryOutcome::Inactive => Vec::<u64>::new(),
                    TurnMemoryOutcome::InvalidReplyStart => Vec::<u64>::new(),
                    TurnMemoryOutcome::SnapshotRefusedTurnMemory => Vec::<u64>::new(),
                }
            }
        }
    }
}

/// Generated from `R4.SpiralCoreFbs` (`src/R4/SpiralCoreFbs.lex.tex`).
pub mod r4_spiral_core_fbs {
    /// Inductive type `R4.SpiralCoreFbs.Sqrt5BoundOutcome`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Sqrt5BoundOutcome {
        /// Variant `definitelyBelow`.
        DefinitelyBelow,
        /// Variant `definitelyNotBelow`.
        DefinitelyNotBelow,
        /// Variant `ambiguousBand`.
        AmbiguousBand,
        /// Variant `zeroDenominator`.
        ZeroDenominator,
    }

    /// Inductive type `R4.SpiralCoreFbs.SnapOutcome`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum SnapOutcome {
        /// Variant `selected`.
        Selected(u64, u64, u64),
        /// Variant `invalidRootIndex`.
        InvalidRootIndex,
        /// Variant `emptyCandidates`.
        EmptyCandidates,
    }

    /// Inductive type `R4.SpiralCoreFbs.NormOutcome`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum NormOutcome {
        /// Variant `unitFastPath`.
        UnitFastPath(u64),
        /// Variant `requiresNormalization`.
        RequiresNormalization(u64, u64),
        /// Variant `zeroQuaternion`.
        ZeroQuaternion,
    }

    /// Inductive type `R4.SpiralCoreFbs.ContainerProbeOutcome`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ContainerProbeOutcome {
        /// Variant `admittedStack`.
        AdmittedStack(u64),
        /// Variant `admittedDiscrete`.
        AdmittedDiscrete(u64),
        /// Variant `refusedRetiredUor4`.
        RefusedRetiredUor4,
        /// Variant `refusedRetiredUorLut1`.
        RefusedRetiredUorLut1,
        /// Variant `refusedBadMagic`.
        RefusedBadMagic,
        /// Variant `refusedUnalignedData`.
        RefusedUnalignedData,
        /// Variant `refusedUnsupportedSchema`.
        RefusedUnsupportedSchema,
        /// Variant `refusedInvalidHeader`.
        RefusedInvalidHeader,
    }

    /// Structure `R4.SpiralCoreFbs.IcosianCandidate`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct IcosianCandidate {
        /// Field `rootIndex`.
        pub root_index: u64,
        /// Field `s0`.
        pub s0: u64,
        /// Field `s1`.
        pub s1: u64,
        /// Field `s2`.
        pub s2: u64,
        /// Field `s3`.
        pub s3: u64,
    }

    /// Function `R4.SpiralCoreFbs.subtractNat`.
    pub fn subtract_nat(left: u64, right: u64) -> u64 {
        (left.clone()).saturating_sub(right.clone())
    }

    /// Function `R4.SpiralCoreFbs.scaleNat`.
    pub fn scale_nat(value: u64, times: u64) -> u64 {
        (value.clone()).saturating_mul(times.clone())
    }

    /// Function `R4.SpiralCoreFbs.absDiff`.
    pub fn abs_diff(a: u64, b: u64) -> u64 {
        (subtract_nat(a.clone(), b.clone())).saturating_add(subtract_nat(b.clone(), a.clone()))
    }

    /// Function `R4.SpiralCoreFbs.belowDSqrt5`.
    pub fn below_d_sqrt5(q: u64, d: u64) -> Sqrt5BoundOutcome {
        if ((d.clone()) == (0u64)) {
            Sqrt5BoundOutcome::ZeroDenominator
        } else {
            {
                let lower: u64 = scale_nat(d.clone(), 2289u64);
                if ((scale_nat(q.clone(), 1024u64)) <= (lower.clone())) {
                    Sqrt5BoundOutcome::DefinitelyBelow
                } else {
                    {
                        let upper: u64 = scale_nat(d.clone(), 1145u64);
                        if ((upper.clone()) <= (scale_nat(q.clone(), 512u64))) {
                            Sqrt5BoundOutcome::DefinitelyNotBelow
                        } else {
                            Sqrt5BoundOutcome::AmbiguousBand
                        }
                    }
                }
            }
        }
    }

    /// Function `R4.SpiralCoreFbs.resolveSqrt5Exact`.
    pub fn resolve_sqrt5_exact(q: u64, d: u64) -> bool {
        match below_d_sqrt5(q.clone(), d.clone()) {
            Sqrt5BoundOutcome::DefinitelyBelow => true,
            Sqrt5BoundOutcome::DefinitelyNotBelow => false,
            Sqrt5BoundOutcome::AmbiguousBand => {
                ((scale_nat(q.clone(), q.clone()))
                    < (scale_nat(scale_nat(d.clone(), d.clone()), 5u64)))
            }
            Sqrt5BoundOutcome::ZeroDenominator => false,
        }
    }

    /// Function `R4.SpiralCoreFbs.snapCompatibleScore`.
    pub fn snap_compatible_score(
        root_index: u64,
        s0: u64,
        s1: u64,
        s2: u64,
        s3: u64,
    ) -> SnapOutcome {
        if ((root_index.clone()) < (2u64)) {
            SnapOutcome::Selected(
                root_index.clone(),
                (s0.clone()).saturating_add(s0.clone()),
                0u64,
            )
        } else {
            if ((root_index.clone()) < (4u64)) {
                SnapOutcome::Selected(
                    root_index.clone(),
                    (s1.clone()).saturating_add(s1.clone()),
                    0u64,
                )
            } else {
                if ((root_index.clone()) < (6u64)) {
                    SnapOutcome::Selected(
                        root_index.clone(),
                        (s2.clone()).saturating_add(s2.clone()),
                        0u64,
                    )
                } else {
                    if ((root_index.clone()) < (8u64)) {
                        SnapOutcome::Selected(
                            root_index.clone(),
                            (s3.clone()).saturating_add(s3.clone()),
                            0u64,
                        )
                    } else {
                        if ((root_index.clone()) < (24u64)) {
                            SnapOutcome::Selected(
                                root_index.clone(),
                                ((s0.clone()).saturating_add(s1.clone()))
                                    .saturating_add((s2.clone()).saturating_add(s3.clone())),
                                0u64,
                            )
                        } else {
                            if ((root_index.clone()) < (120u64)) {
                                SnapOutcome::Selected(
                                    root_index.clone(),
                                    subtract_nat(s1.clone(), s2.clone()),
                                    (s0.clone()).saturating_add(s2.clone()),
                                )
                            } else {
                                SnapOutcome::InvalidRootIndex
                            }
                        }
                    }
                }
            }
        }
    }

    /// Function `R4.SpiralCoreFbs.phiScoreStrictlyBeats`.
    pub fn phi_score_strictly_beats(cand_a: u64, cand_b: u64, best_a: u64, best_b: u64) -> bool {
        if (((best_a.clone()) <= (cand_a.clone())) && ((best_b.clone()) <= (cand_b.clone()))) {
            (((best_a.clone()) < (cand_a.clone())) || ((best_b.clone()) < (cand_b.clone())))
        } else {
            if (((cand_a.clone()) < (best_a.clone())) && ((best_b.clone()) < (cand_b.clone()))) {
                {
                    let da: u64 = subtract_nat(best_a.clone(), cand_a.clone());
                    {
                        let db: u64 = subtract_nat(cand_b.clone(), best_b.clone());
                        {
                            let two_da: u64 = (da.clone()).saturating_add(da.clone());
                            if ((two_da.clone()) <= (db.clone())) {
                                true
                            } else {
                                resolve_sqrt5_exact(
                                    subtract_nat(two_da.clone(), db.clone()),
                                    db.clone(),
                                )
                            }
                        }
                    }
                }
            } else {
                if (((best_a.clone()) < (cand_a.clone())) && ((cand_b.clone()) < (best_b.clone())))
                {
                    {
                        let da: u64 = subtract_nat(cand_a.clone(), best_a.clone());
                        {
                            let db: u64 = subtract_nat(best_b.clone(), cand_b.clone());
                            {
                                let two_da: u64 = (da.clone()).saturating_add(da.clone());
                                if ((two_da.clone()) <= (db.clone())) {
                                    false
                                } else {
                                    (!(resolve_sqrt5_exact(
                                        subtract_nat(two_da.clone(), db.clone()),
                                        db.clone(),
                                    )))
                                }
                            }
                        }
                    }
                } else {
                    false
                }
            }
        }
    }

    /// Function `R4.SpiralCoreFbs.selectIcosianTail`.
    pub fn select_icosian_tail(
        best_root: u64,
        best_a: u64,
        best_b: u64,
        s0: u64,
        s1: u64,
        s2: u64,
        s3: u64,
        roots: Vec<u64>,
    ) -> SnapOutcome {
        {
            let __scrut = roots.clone();
            if __scrut.is_empty() {
                SnapOutcome::Selected(best_root.clone(), best_a.clone(), best_b.clone())
            } else {
                let head_root = __scrut[0].clone();
                let tail_roots = __scrut[1..].to_vec();
                match snap_compatible_score(
                    head_root.clone(),
                    s0.clone(),
                    s1.clone(),
                    s2.clone(),
                    s3.clone(),
                ) {
                    SnapOutcome::Selected(r_idx, cand_a, cand_b) => {
                        if phi_score_strictly_beats(
                            cand_a.clone(),
                            cand_b.clone(),
                            best_a.clone(),
                            best_b.clone(),
                        ) {
                            select_icosian_tail(
                                r_idx.clone(),
                                cand_a.clone(),
                                cand_b.clone(),
                                s0.clone(),
                                s1.clone(),
                                s2.clone(),
                                s3.clone(),
                                tail_roots.clone(),
                            )
                        } else {
                            select_icosian_tail(
                                best_root.clone(),
                                best_a.clone(),
                                best_b.clone(),
                                s0.clone(),
                                s1.clone(),
                                s2.clone(),
                                s3.clone(),
                                tail_roots.clone(),
                            )
                        }
                    }
                    SnapOutcome::InvalidRootIndex => SnapOutcome::InvalidRootIndex,
                    SnapOutcome::EmptyCandidates => SnapOutcome::EmptyCandidates,
                }
            }
        }
    }

    /// Function `R4.SpiralCoreFbs.selectIcosianRoot`.
    pub fn select_icosian_root(
        s0: u64,
        s1: u64,
        s2: u64,
        s3: u64,
        candidate_roots: Vec<u64>,
    ) -> SnapOutcome {
        {
            let __scrut = candidate_roots.clone();
            if __scrut.is_empty() {
                SnapOutcome::EmptyCandidates
            } else {
                let first_root = __scrut[0].clone();
                let rest_roots = __scrut[1..].to_vec();
                match snap_compatible_score(
                    first_root.clone(),
                    s0.clone(),
                    s1.clone(),
                    s2.clone(),
                    s3.clone(),
                ) {
                    SnapOutcome::Selected(r_idx, first_a, first_b) => select_icosian_tail(
                        r_idx.clone(),
                        first_a.clone(),
                        first_b.clone(),
                        s0.clone(),
                        s1.clone(),
                        s2.clone(),
                        s3.clone(),
                        rest_roots.clone(),
                    ),
                    SnapOutcome::InvalidRootIndex => SnapOutcome::InvalidRootIndex,
                    SnapOutcome::EmptyCandidates => SnapOutcome::EmptyCandidates,
                }
            }
        }
    }

    /// Function `R4.SpiralCoreFbs.hamiltonLaneX`.
    pub fn hamilton_lane_x(
        a0: u64,
        a1: u64,
        a2: u64,
        a3: u64,
        b0: u64,
        b1: u64,
        b2: u64,
        b3: u64,
    ) -> u64 {
        subtract_nat(
            ((scale_nat(a0.clone(), b1.clone())).saturating_add(scale_nat(a1.clone(), b0.clone())))
                .saturating_add(scale_nat(a2.clone(), b3.clone())),
            scale_nat(a3.clone(), b2.clone()),
        )
    }

    /// Function `R4.SpiralCoreFbs.quaternionNormSq`.
    pub fn quaternion_norm_sq(w: u64, x: u64, y: u64, z: u64) -> u64 {
        ((scale_nat(w.clone(), w.clone())).saturating_add(scale_nat(x.clone(), x.clone())))
            .saturating_add(
                (scale_nat(y.clone(), y.clone())).saturating_add(scale_nat(z.clone(), z.clone())),
            )
    }

    /// Function `R4.SpiralCoreFbs.checkUnitNorm`.
    pub fn check_unit_norm(
        w: u64,
        x: u64,
        y: u64,
        z: u64,
        unit_target_sq: u64,
        tolerance: u64,
        seed_root: u64,
    ) -> NormOutcome {
        {
            let norm_sq: u64 = quaternion_norm_sq(w.clone(), x.clone(), y.clone(), z.clone());
            if ((norm_sq.clone()) == (0u64)) {
                NormOutcome::ZeroQuaternion
            } else {
                if ((abs_diff(norm_sq.clone(), unit_target_sq.clone())) <= (tolerance.clone())) {
                    NormOutcome::UnitFastPath(norm_sq.clone())
                } else {
                    NormOutcome::RequiresNormalization(norm_sq.clone(), seed_root.clone())
                }
            }
        }
    }

    /// Function `R4.SpiralCoreFbs.probeModelSource`.
    pub fn probe_model_source(
        magic_tag: u64,
        schema_kind: u64,
        schema_ver: u64,
        data_offset_mod64: u64,
        table_count: u64,
    ) -> ContainerProbeOutcome {
        if ((magic_tag.clone()) == (2u64)) {
            ContainerProbeOutcome::RefusedRetiredUor4
        } else {
            if ((magic_tag.clone()) == (3u64)) {
                ContainerProbeOutcome::RefusedRetiredUorLut1
            } else {
                if (!((magic_tag.clone()) == (1u64))) {
                    ContainerProbeOutcome::RefusedBadMagic
                } else {
                    if (!((data_offset_mod64.clone()) == (0u64))) {
                        ContainerProbeOutcome::RefusedUnalignedData
                    } else {
                        if ((schema_kind.clone()) == (1u64)) {
                            if (((1u64) <= (schema_ver.clone()))
                                && ((schema_ver.clone()) <= (8u64)))
                            {
                                ContainerProbeOutcome::AdmittedStack(schema_ver.clone())
                            } else {
                                ContainerProbeOutcome::RefusedUnsupportedSchema
                            }
                        } else {
                            if ((schema_kind.clone()) == (2u64)) {
                                if (((schema_ver.clone()) == (1u64))
                                    && (((table_count.clone()) == (4u64))
                                        || ((table_count.clone()) == (19u64))))
                                {
                                    ContainerProbeOutcome::AdmittedDiscrete(table_count.clone())
                                } else {
                                    ContainerProbeOutcome::RefusedInvalidHeader
                                }
                            } else {
                                ContainerProbeOutcome::RefusedUnsupportedSchema
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Generated from `R4.DiscreteTrunk` (`src/R4/DiscreteTrunk.lex.tex`).
pub mod r4_discrete_trunk {
    /// Inductive type `R4.DiscreteTrunk.TrunkOutcome`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum TrunkOutcome {
        /// Variant `predicted`.
        Predicted(u64, u64),
        /// Variant `causalSuccessorHit`.
        CausalSuccessorHit(u64),
        /// Variant `causalSuccessorNone`.
        CausalSuccessorNone,
        /// Variant `packedCode`.
        PackedCode(u64),
        /// Variant `invalidRootIndex`.
        InvalidRootIndex,
        /// Variant `emptyPrefix`.
        EmptyPrefix,
        /// Variant `tokenOutOfBounds`.
        TokenOutOfBounds,
        /// Variant `invalidTableCount`.
        InvalidTableCount,
        /// Variant `unexpectedTrailingBytes`.
        UnexpectedTrailingBytes,
    }

    /// Structure `R4.DiscreteTrunk.PrefixEntry`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct PrefixEntry {
        /// Field `token`.
        pub token: u64,
        /// Field `rootCode`.
        pub root_code: u64,
    }

    /// Structure `R4.DiscreteTrunk.SparseTriple`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SparseTriple {
        /// Field `key`.
        pub key: u64,
        /// Field `targetToken`.
        pub target_token: u64,
        /// Field `count`.
        pub count: u64,
    }

    /// Structure `R4.DiscreteTrunk.TrunkCandidate`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TrunkCandidate {
        /// Field `token`.
        pub token: u64,
        /// Field `cQr`.
        pub c_qr: u64,
        /// Field `cQc`.
        pub c_qc: u64,
        /// Field `cRr`.
        pub c_rr: u64,
        /// Field `cR`.
        pub c_r: u64,
        /// Field `cQ`.
        pub c_q: u64,
        /// Field `unigramCount`.
        pub unigram_count: u64,
    }

    /// Function `R4.DiscreteTrunk.subtractNat`.
    pub fn subtract_nat(left: u64, right: u64) -> u64 {
        (left.clone()).saturating_sub(right.clone())
    }

    /// Function `R4.DiscreteTrunk.scaleNat`.
    pub fn scale_nat(value: u64, times: u64) -> u64 {
        (value.clone()).saturating_mul(times.clone())
    }

    /// Function `R4.DiscreteTrunk.absDiff`.
    pub fn abs_diff(a: u64, b: u64) -> u64 {
        (subtract_nat(a.clone(), b.clone())).saturating_add(subtract_nat(b.clone(), a.clone()))
    }

    /// Function `R4.DiscreteTrunk.pack4BlockRoots`.
    pub fn pack4_block_roots(r0: u64, r1: u64, r2: u64, r3: u64) -> TrunkOutcome {
        if ((((r0.clone()) < (16u64)) && ((r1.clone()) < (16u64)))
            && (((r2.clone()) < (16u64)) && ((r3.clone()) < (16u64))))
        {
            {
                let p01: u64 = (scale_nat(r0.clone(), 16u64)).saturating_add(r1.clone());
                {
                    let p012: u64 = (scale_nat(p01.clone(), 16u64)).saturating_add(r2.clone());
                    TrunkOutcome::PackedCode(
                        (scale_nat(p012.clone(), 16u64)).saturating_add(r3.clone()),
                    )
                }
            }
        } else {
            TrunkOutcome::InvalidRootIndex
        }
    }

    /// Function `R4.DiscreteTrunk.computeRecComparisonCode`.
    pub fn compute_rec_comparison_code(
        lag1_eq: bool,
        lag2_eq: bool,
        lag3_eq: bool,
        lag4_eq: bool,
    ) -> u64 {
        ((if lag1_eq.clone() { 1u64 } else { 0u64 }).saturating_add(if lag2_eq.clone() {
            2u64
        } else {
            0u64
        }))
        .saturating_add(
            (if lag3_eq.clone() { 4u64 } else { 0u64 }).saturating_add(if lag4_eq.clone() {
                8u64
            } else {
                0u64
            }),
        )
    }

    /// Function `R4.DiscreteTrunk.scanCausalSuccessor`.
    pub fn scan_causal_successor(
        rest: Vec<PrefixEntry>,
        prev_root_code: u64,
        query_root_code: u64,
        vocab_size: u64,
        found_any: bool,
        latest_succ: u64,
    ) -> TrunkOutcome {
        {
            let __scrut = rest.clone();
            if __scrut.is_empty() {
                if found_any.clone() {
                    TrunkOutcome::CausalSuccessorHit(latest_succ.clone())
                } else {
                    TrunkOutcome::CausalSuccessorNone
                }
            } else {
                let next_entry = __scrut[0].clone();
                let after_next = __scrut[1..].to_vec();
                if ((vocab_size.clone()) <= ((next_entry.clone()).token.clone())) {
                    TrunkOutcome::TokenOutOfBounds
                } else {
                    if ((prev_root_code.clone()) == (query_root_code.clone())) {
                        scan_causal_successor(
                            after_next.clone(),
                            (next_entry.clone()).root_code.clone(),
                            query_root_code.clone(),
                            vocab_size.clone(),
                            true,
                            (next_entry.clone()).token.clone(),
                        )
                    } else {
                        scan_causal_successor(
                            after_next.clone(),
                            (next_entry.clone()).root_code.clone(),
                            query_root_code.clone(),
                            vocab_size.clone(),
                            found_any.clone(),
                            latest_succ.clone(),
                        )
                    }
                }
            }
        }
    }

    /// Function `R4.DiscreteTrunk.orbitRootCausalSuccessor`.
    pub fn orbit_root_causal_successor(
        prefix_entries: Vec<PrefixEntry>,
        query_root_code: u64,
        vocab_size: u64,
    ) -> TrunkOutcome {
        {
            let __scrut = prefix_entries.clone();
            if __scrut.is_empty() {
                TrunkOutcome::EmptyPrefix
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                if ((vocab_size.clone()) <= ((head.clone()).token.clone())) {
                    TrunkOutcome::TokenOutOfBounds
                } else {
                    scan_causal_successor(
                        tail.clone(),
                        (head.clone()).root_code.clone(),
                        query_root_code.clone(),
                        vocab_size.clone(),
                        false,
                        0u64,
                    )
                }
            }
        }
    }

    /// Function `R4.DiscreteTrunk.lookupSparseTriple`.
    pub fn lookup_sparse_triple(entries: Vec<SparseTriple>, key: u64, target_token: u64) -> u64 {
        {
            let __scrut = entries.clone();
            if __scrut.is_empty() {
                0u64
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                if ((((head.clone()).key.clone()) == (key.clone()))
                    && (((head.clone()).target_token.clone()) == (target_token.clone())))
                {
                    (head.clone()).count.clone()
                } else {
                    lookup_sparse_triple(tail.clone(), key.clone(), target_token.clone())
                }
            }
        }
    }

    /// Function `R4.DiscreteTrunk.level3JointPointerScore`.
    pub fn level3_joint_pointer_score(
        c_qr: u64,
        c_qc: u64,
        c_rr: u64,
        c_r: u64,
        c_q: u64,
        is_pointer_match: bool,
    ) -> u64 {
        (((scale_nat(c_qr.clone(), 64u64)).saturating_add(scale_nat(c_qc.clone(), 32u64)))
            .saturating_add(
                (scale_nat(c_rr.clone(), 8u64)).saturating_add(scale_nat(c_r.clone(), 2u64)),
            ))
        .saturating_add((c_q.clone()).saturating_add(if is_pointer_match.clone() {
            256u64
        } else {
            0u64
        }))
    }

    /// Function `R4.DiscreteTrunk.jointCandidateBeats`.
    pub fn joint_candidate_beats(
        cand_tok: u64,
        cand_score: u64,
        cand_unigram: u64,
        best_tok: u64,
        best_score: u64,
        best_unigram: u64,
    ) -> bool {
        (((best_score.clone()) < (cand_score.clone()))
            || ((((cand_score.clone()) == (best_score.clone()))
                && ((0u64) < (cand_score.clone())))
                && (((best_unigram.clone()) < (cand_unigram.clone()))
                    || (((cand_unigram.clone()) == (best_unigram.clone()))
                        && ((cand_tok.clone()) < (best_tok.clone()))))))
    }

    /// Function `R4.DiscreteTrunk.selectTrunkTail`.
    pub fn select_trunk_tail(
        candidates: Vec<TrunkCandidate>,
        has_pointer: bool,
        pointer_tok: u64,
        best_tok: u64,
        best_score: u64,
        best_unigram: u64,
    ) -> TrunkOutcome {
        {
            let __scrut = candidates.clone();
            if __scrut.is_empty() {
                TrunkOutcome::Predicted(best_tok.clone(), best_score.clone())
            } else {
                let cand = __scrut[0].clone();
                let rest = __scrut[1..].to_vec();
                {
                    let cand_tok: u64 = (cand.clone()).token.clone();
                    {
                        let is_ptr: bool = ((has_pointer.clone())
                            && ((cand_tok.clone()) == (pointer_tok.clone())));
                        {
                            let score: u64 = level3_joint_pointer_score(
                                (cand.clone()).c_qr.clone(),
                                (cand.clone()).c_qc.clone(),
                                (cand.clone()).c_rr.clone(),
                                (cand.clone()).c_r.clone(),
                                (cand.clone()).c_q.clone(),
                                is_ptr.clone(),
                            );
                            {
                                let u: u64 = (cand.clone()).unigram_count.clone();
                                if joint_candidate_beats(
                                    cand_tok.clone(),
                                    score.clone(),
                                    u.clone(),
                                    best_tok.clone(),
                                    best_score.clone(),
                                    best_unigram.clone(),
                                ) {
                                    select_trunk_tail(
                                        rest.clone(),
                                        has_pointer.clone(),
                                        pointer_tok.clone(),
                                        cand_tok.clone(),
                                        score.clone(),
                                        u.clone(),
                                    )
                                } else {
                                    select_trunk_tail(
                                        rest.clone(),
                                        has_pointer.clone(),
                                        pointer_tok.clone(),
                                        best_tok.clone(),
                                        best_score.clone(),
                                        best_unigram.clone(),
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Function `R4.DiscreteTrunk.predictTrunkCandidate`.
    pub fn predict_trunk_candidate(
        candidates: Vec<TrunkCandidate>,
        has_pointer: bool,
        pointer_tok: u64,
        unigram_best_global: u64,
    ) -> TrunkOutcome {
        select_trunk_tail(
            candidates.clone(),
            has_pointer.clone(),
            pointer_tok.clone(),
            unigram_best_global.clone(),
            0u64,
            0u64,
        )
    }

    /// Function `R4.DiscreteTrunk.validateTrunkHeader`.
    pub fn validate_trunk_header(
        is_trunk_schema: bool,
        table_count: u64,
        block_count: u64,
        trailing_bytes: u64,
    ) -> TrunkOutcome {
        if (!((trailing_bytes.clone()) == (0u64))) {
            TrunkOutcome::UnexpectedTrailingBytes
        } else {
            if is_trunk_schema.clone() {
                if (((table_count.clone()) == (19u64)) && ((block_count.clone()) == (4u64))) {
                    TrunkOutcome::PackedCode(19u64)
                } else {
                    TrunkOutcome::InvalidTableCount
                }
            } else {
                if ((table_count.clone()) == (4u64)) {
                    TrunkOutcome::PackedCode(4u64)
                } else {
                    TrunkOutcome::InvalidTableCount
                }
            }
        }
    }
}

/// Generated from `R4.DurableMemory` (`src/R4/DurableMemory.lex.tex`).
pub mod r4_durable_memory {
    /// Inductive type `R4.DurableMemory.DurableReadOutcome`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum DurableReadOutcome {
        /// Variant `quoted`.
        Quoted(u64, u64, u64),
        /// Variant `refusedUnstated`.
        RefusedUnstated,
        /// Variant `refusedRetracted`.
        RefusedRetracted(u64),
        /// Variant `refusedScopeMismatch`.
        RefusedScopeMismatch,
        /// Variant `refusedBadMagic`.
        RefusedBadMagic,
        /// Variant `refusedTruncatedEnvelope`.
        RefusedTruncatedEnvelope,
        /// Variant `refusedVarintOverflow`.
        RefusedVarintOverflow,
        /// Variant `evictedSlot`.
        EvictedSlot(u64),
    }

    /// Inductive type `R4.DurableMemory.SketchGateOutcome`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum SketchGateOutcome {
        /// Variant `admitted`.
        Admitted(u64, u64),
        /// Variant `abstainedOutOfManifold`.
        AbstainedOutOfManifold(u64),
        /// Variant `invalidSketchDimension`.
        InvalidSketchDimension,
    }

    /// Structure `R4.DurableMemory.IdentityScopeRecord`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct IdentityScopeRecord {
        /// Field `userId`.
        pub user_id: u64,
        /// Field `projectId`.
        pub project_id: u64,
        /// Field `sessionId`.
        pub session_id: u64,
        /// Field `fieldByteLen`.
        pub field_byte_len: u64,
    }

    /// Structure `R4.DurableMemory.FactSlot`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct FactSlot {
        /// Field `userId`.
        pub user_id: u64,
        /// Field `projectId`.
        pub project_id: u64,
        /// Field `sessionId`.
        pub session_id: u64,
        /// Field `ownerId`.
        pub owner_id: u64,
        /// Field `relationId`.
        pub relation_id: u64,
        /// Field `actionCode`.
        pub action_code: u64,
        /// Field `version`.
        pub version: u64,
        /// Field `currentValue`.
        pub current_value: u64,
        /// Field `previousValue`.
        pub previous_value: u64,
    }

    /// Structure `R4.DurableMemory.PrimeDirectoryEntry`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct PrimeDirectoryEntry {
        /// Field `slotId`.
        pub slot_id: u64,
        /// Field `primes`.
        pub primes: Vec<u64>,
        /// Field `currentValue`.
        pub current_value: u64,
        /// Field `previousValue`.
        pub previous_value: u64,
        /// Field `version`.
        pub version: u64,
        /// Field `isEvicted`.
        pub is_evicted: bool,
        /// Field `evictedAtSeq`.
        pub evicted_at_seq: u64,
    }

    /// Function `R4.DurableMemory.subtractNat`.
    pub fn subtract_nat(left: u64, right: u64) -> u64 {
        (left.clone()).saturating_sub(right.clone())
    }

    /// Function `R4.DurableMemory.scaleNat`.
    pub fn scale_nat(value: u64, times: u64) -> u64 {
        (value.clone()).saturating_mul(times.clone())
    }

    /// Function `R4.DurableMemory.absDiff`.
    pub fn abs_diff(a: u64, b: u64) -> u64 {
        (subtract_nat(a.clone(), b.clone())).saturating_add(subtract_nat(b.clone(), a.clone()))
    }

    /// Function `R4.DurableMemory.scopeIsValid`.
    pub fn scope_is_valid(scope: IdentityScopeRecord) -> bool {
        ((((0u64) < ((scope.clone()).user_id.clone()))
            && ((0u64) < ((scope.clone()).project_id.clone())))
            && (((0u64) < ((scope.clone()).session_id.clone()))
                && (((0u64) < ((scope.clone()).field_byte_len.clone()))
                    && (((scope.clone()).field_byte_len.clone()) <= (256u64)))))
    }

    /// Function `R4.DurableMemory.scopesMatch`.
    pub fn scopes_match(query_scope: IdentityScopeRecord, slot: FactSlot) -> bool {
        ((((query_scope.clone()).user_id.clone()) == ((slot.clone()).user_id.clone()))
            && ((((query_scope.clone()).project_id.clone())
                == ((slot.clone()).project_id.clone()))
                && (((query_scope.clone()).session_id.clone())
                    == ((slot.clone()).session_id.clone()))))
    }

    /// Function `R4.DurableMemory.lookupScopedFactSlots`.
    pub fn lookup_scoped_fact_slots(
        slots: Vec<FactSlot>,
        query_scope: IdentityScopeRecord,
        owner_id: u64,
        relation_id: u64,
        saw_cross_scope: bool,
    ) -> DurableReadOutcome {
        {
            let __scrut = slots.clone();
            if __scrut.is_empty() {
                if saw_cross_scope.clone() {
                    DurableReadOutcome::RefusedScopeMismatch
                } else {
                    DurableReadOutcome::RefusedUnstated
                }
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                if ((((head.clone()).owner_id.clone()) == (owner_id.clone()))
                    && (((head.clone()).relation_id.clone()) == (relation_id.clone())))
                {
                    if scopes_match(query_scope.clone(), head.clone()) {
                        if (((head.clone()).action_code.clone()) == (3u64)) {
                            DurableReadOutcome::RefusedRetracted((head.clone()).version.clone())
                        } else {
                            DurableReadOutcome::Quoted(
                                (head.clone()).current_value.clone(),
                                (head.clone()).version.clone(),
                                (head.clone()).action_code.clone(),
                            )
                        }
                    } else {
                        lookup_scoped_fact_slots(
                            tail.clone(),
                            query_scope.clone(),
                            owner_id.clone(),
                            relation_id.clone(),
                            true,
                        )
                    }
                } else {
                    lookup_scoped_fact_slots(
                        tail.clone(),
                        query_scope.clone(),
                        owner_id.clone(),
                        relation_id.clone(),
                        saw_cross_scope.clone(),
                    )
                }
            }
        }
    }

    /// Function `R4.DurableMemory.readScopedFact`.
    pub fn read_scoped_fact(
        slots: Vec<FactSlot>,
        query_scope: IdentityScopeRecord,
        owner_id: u64,
        relation_id: u64,
    ) -> DurableReadOutcome {
        if (!(scope_is_valid(query_scope.clone()))) {
            DurableReadOutcome::RefusedScopeMismatch
        } else {
            lookup_scoped_fact_slots(
                slots.clone(),
                query_scope.clone(),
                owner_id.clone(),
                relation_id.clone(),
                false,
            )
        }
    }

    /// Function `R4.DurableMemory.validateSessionEnvelope`.
    pub fn validate_session_envelope(
        magic_tag: u64,
        version: u64,
        varint_shift: u64,
        remaining_bytes: u64,
        needed_bytes: u64,
    ) -> DurableReadOutcome {
        if (!(((magic_tag.clone()) == (1u64)) || ((magic_tag.clone()) == (2u64)))) {
            DurableReadOutcome::RefusedBadMagic
        } else {
            if ((64u64) <= (varint_shift.clone())) {
                DurableReadOutcome::RefusedVarintOverflow
            } else {
                if ((remaining_bytes.clone()) < (needed_bytes.clone())) {
                    DurableReadOutcome::RefusedTruncatedEnvelope
                } else {
                    DurableReadOutcome::Quoted(
                        needed_bytes.clone(),
                        version.clone(),
                        magic_tag.clone(),
                    )
                }
            }
        }
    }

    /// Function `R4.DurableMemory.containsPrime`.
    pub fn contains_prime(target: u64, primes: Vec<u64>) -> bool {
        {
            let __scrut = primes.clone();
            if __scrut.is_empty() {
                false
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                if ((head.clone()) == (target.clone())) {
                    true
                } else {
                    if ((target.clone()) < (head.clone())) {
                        false
                    } else {
                        contains_prime(target.clone(), tail.clone())
                    }
                }
            }
        }
    }

    /// Function `R4.DurableMemory.sortedMergeGcdOverlap`.
    pub fn sorted_merge_gcd_overlap(left_primes: Vec<u64>, right_primes: Vec<u64>) -> u64 {
        {
            let __scrut = left_primes.clone();
            if __scrut.is_empty() {
                0u64
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                (if contains_prime(head.clone(), right_primes.clone()) {
                    1u64
                } else {
                    0u64
                })
                .saturating_add(sorted_merge_gcd_overlap(tail.clone(), right_primes.clone()))
            }
        }
    }

    /// Function `R4.DurableMemory.lookupPrimeDirectoryTail`.
    pub fn lookup_prime_directory_tail(
        entries: Vec<PrimeDirectoryEntry>,
        query_primes: Vec<u64>,
        want_previous: bool,
        best_overlap: u64,
        best_val: u64,
        best_ver: u64,
        best_evicted: bool,
        best_evicted_seq: u64,
    ) -> DurableReadOutcome {
        {
            let __scrut = entries.clone();
            if __scrut.is_empty() {
                if ((best_overlap.clone()) == (0u64)) {
                    DurableReadOutcome::RefusedUnstated
                } else {
                    if best_evicted.clone() {
                        DurableReadOutcome::EvictedSlot(best_evicted_seq.clone())
                    } else {
                        DurableReadOutcome::Quoted(
                            best_val.clone(),
                            best_ver.clone(),
                            best_overlap.clone(),
                        )
                    }
                }
            } else {
                let entry = __scrut[0].clone();
                let rest = __scrut[1..].to_vec();
                {
                    let overlap: u64 = sorted_merge_gcd_overlap(
                        query_primes.clone(),
                        (entry.clone()).primes.clone(),
                    );
                    if ((best_overlap.clone()) < (overlap.clone())) {
                        {
                            let chosen_val: u64 = if want_previous.clone() {
                                (entry.clone()).previous_value.clone()
                            } else {
                                (entry.clone()).current_value.clone()
                            };
                            lookup_prime_directory_tail(
                                rest.clone(),
                                query_primes.clone(),
                                want_previous.clone(),
                                overlap.clone(),
                                chosen_val.clone(),
                                (entry.clone()).version.clone(),
                                (entry.clone()).is_evicted.clone(),
                                (entry.clone()).evicted_at_seq.clone(),
                            )
                        }
                    } else {
                        lookup_prime_directory_tail(
                            rest.clone(),
                            query_primes.clone(),
                            want_previous.clone(),
                            best_overlap.clone(),
                            best_val.clone(),
                            best_ver.clone(),
                            best_evicted.clone(),
                            best_evicted_seq.clone(),
                        )
                    }
                }
            }
        }
    }

    /// Function `R4.DurableMemory.resolvePrimeAddress`.
    pub fn resolve_prime_address(
        entries: Vec<PrimeDirectoryEntry>,
        query_primes: Vec<u64>,
        want_previous: bool,
    ) -> DurableReadOutcome {
        lookup_prime_directory_tail(
            entries.clone(),
            query_primes.clone(),
            want_previous.clone(),
            0u64,
            0u64,
            0u64,
            false,
            0u64,
        )
    }

    /// Function `R4.DurableMemory.manifoldSketchGate`.
    pub fn manifold_sketch_gate(overlap_bits: u64, sketch_dim: u64) -> SketchGateOutcome {
        if ((!((sketch_dim.clone()) == (4096u64))) || ((4096u64) < (overlap_bits.clone()))) {
            SketchGateOutcome::InvalidSketchDimension
        } else {
            if ((2144u64) <= (overlap_bits.clone())) {
                SketchGateOutcome::Admitted(
                    overlap_bits.clone(),
                    subtract_nat(overlap_bits.clone(), 2048u64),
                )
            } else {
                SketchGateOutcome::AbstainedOutOfManifold(overlap_bits.clone())
            }
        }
    }

    /// Function `R4.DurableMemory.blendHierarchicalSketchNumerator`.
    pub fn blend_hierarchical_sketch_numerator(local_val: u64, parent_val: u64) -> u64 {
        (scale_nat(local_val.clone(), 13u64)).saturating_add(scale_nat(parent_val.clone(), 3u64))
    }
}

/// Generated from `R4.ExactArithmetic` (`src/R4/ExactArithmetic.lex.tex`).
pub mod r4_exact_arithmetic {
    /// Inductive type `R4.ExactArithmetic.ArithmeticOutcome`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ArithmeticOutcome {
        /// Variant `placeStep`.
        PlaceStep(u64, u64, u64, u64),
        /// Variant `multiDigitResult`.
        MultiDigitResult(Vec<u64>, u64),
        /// Variant `invalidRadix`.
        InvalidRadix,
        /// Variant `digitOutOfRange`.
        DigitOutOfRange,
        /// Variant `invalidCarryTrit`.
        InvalidCarryTrit,
        /// Variant `placeLimitExceeded`.
        PlaceLimitExceeded,
        /// Variant `ablationMutedMemory`.
        AblationMutedMemory,
    }

    /// Inductive type `R4.ExactArithmetic.LikelihoodOutcome`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum LikelihoodOutcome {
        /// Variant `massAllocated`.
        MassAllocated(u64, u64),
        /// Variant `bpbFraction`.
        BpbFraction(u64, u64),
        /// Variant `refusedEmptyScores`.
        RefusedEmptyScores,
        /// Variant `refusedMassZero`.
        RefusedMassZero,
        /// Variant `refusedZeroBytes`.
        RefusedZeroBytes,
    }

    /// Inductive type `R4.ExactArithmetic.NafOutcome`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum NafOutcome {
        /// Variant `acceptedCanonical`.
        AcceptedCanonical(u64, u64),
        /// Variant `invalidNonMinimalUvar`.
        InvalidNonMinimalUvar,
        /// Variant `invalidBadMagic`.
        InvalidBadMagic,
        /// Variant `unresolved`.
        Unresolved,
        /// Variant `unsupportedDomain`.
        UnsupportedDomain,
        /// Variant `resourceRefusal`.
        ResourceRefusal,
        /// Variant `policyOutOfDomain`.
        PolicyOutOfDomain,
    }

    /// Structure `R4.ExactArithmetic.DigitPair`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct DigitPair {
        /// Field `leftDigit`.
        pub left_digit: u64,
        /// Field `rightDigit`.
        pub right_digit: u64,
    }

    /// Function `R4.ExactArithmetic.subtractNat`.
    pub fn subtract_nat(left: u64, right: u64) -> u64 {
        (left.clone()).saturating_sub(right.clone())
    }

    /// Function `R4.ExactArithmetic.scaleNat`.
    pub fn scale_nat(value: u64, times: u64) -> u64 {
        (value.clone()).saturating_mul(times.clone())
    }

    /// Function `R4.ExactArithmetic.absDiff`.
    pub fn abs_diff(a: u64, b: u64) -> u64 {
        (subtract_nat(a.clone(), b.clone())).saturating_add(subtract_nat(b.clone(), a.clone()))
    }

    /// Function `R4.ExactArithmetic.radixIsValid`.
    pub fn radix_is_valid(radix: u64) -> bool {
        (((2u64) <= (radix.clone())) && ((radix.clone()) <= (16u64)))
    }

    /// Function `R4.ExactArithmetic.cl06StepActionId`.
    pub fn cl06_step_action_id(left_digit: u64, out_digit: u64) -> u64 {
        if ((left_digit.clone()) == (out_digit.clone())) {
            0u64
        } else {
            ((scale_nat(left_digit.clone(), 3u64)).saturating_add(out_digit.clone()))
                .saturating_add(1u64)
        }
    }

    /// Function `R4.ExactArithmetic.radixPlaceAddStep`.
    pub fn radix_place_add_step(
        radix: u64,
        place: u64,
        left_digit: u64,
        right_digit: u64,
        carry_in: u64,
    ) -> ArithmeticOutcome {
        if (!(radix_is_valid(radix.clone()))) {
            ArithmeticOutcome::InvalidRadix
        } else {
            if ((32u64) <= (place.clone())) {
                ArithmeticOutcome::PlaceLimitExceeded
            } else {
                if (((radix.clone()) <= (left_digit.clone()))
                    || ((radix.clone()) <= (right_digit.clone())))
                {
                    ArithmeticOutcome::DigitOutOfRange
                } else {
                    if ((1u64) < (carry_in.clone())) {
                        ArithmeticOutcome::InvalidCarryTrit
                    } else {
                        {
                            let raw_sum: u64 = ((left_digit.clone())
                                .saturating_add(right_digit.clone()))
                            .saturating_add(carry_in.clone());
                            {
                                let has_carry: bool = ((radix.clone()) <= (raw_sum.clone()));
                                {
                                    let out_digit: u64 = if has_carry.clone() {
                                        subtract_nat(raw_sum.clone(), radix.clone())
                                    } else {
                                        raw_sum.clone()
                                    };
                                    {
                                        let carry_out: u64 =
                                            if has_carry.clone() { 1u64 } else { 0u64 };
                                        {
                                            let act_id: u64 = cl06_step_action_id(
                                                left_digit.clone(),
                                                out_digit.clone(),
                                            );
                                            {
                                                let inv_id: u64 = cl06_step_action_id(
                                                    out_digit.clone(),
                                                    left_digit.clone(),
                                                );
                                                ArithmeticOutcome::PlaceStep(
                                                    out_digit.clone(),
                                                    carry_out.clone(),
                                                    act_id.clone(),
                                                    inv_id.clone(),
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Function `R4.ExactArithmetic.evaluateAblationPlaceStep`.
    pub fn evaluate_ablation_place_step(
        ablation_arm: u64,
        radix: u64,
        place: u64,
        left_digit: u64,
        right_digit: u64,
        carry_in: u64,
    ) -> ArithmeticOutcome {
        if (((ablation_arm.clone()) == (2u64)) || ((ablation_arm.clone()) == (3u64))) {
            ArithmeticOutcome::AblationMutedMemory
        } else {
            {
                let effective_carry: u64 = if ((ablation_arm.clone()) == (1u64)) {
                    0u64
                } else {
                    carry_in.clone()
                };
                radix_place_add_step(
                    radix.clone(),
                    place.clone(),
                    left_digit.clone(),
                    right_digit.clone(),
                    effective_carry.clone(),
                )
            }
        }
    }

    /// Function `R4.ExactArithmetic.multiDigitAddLoop`.
    pub fn multi_digit_add_loop(
        radix: u64,
        place: u64,
        pairs: Vec<DigitPair>,
        carry_in: u64,
    ) -> ArithmeticOutcome {
        {
            let __scrut = pairs.clone();
            if __scrut.is_empty() {
                ArithmeticOutcome::MultiDigitResult(Vec::<u64>::new(), carry_in.clone())
            } else {
                let head = __scrut[0].clone();
                let tail = __scrut[1..].to_vec();
                match radix_place_add_step(
                    radix.clone(),
                    place.clone(),
                    (head.clone()).left_digit.clone(),
                    (head.clone()).right_digit.clone(),
                    carry_in.clone(),
                ) {
                    ArithmeticOutcome::PlaceStep(out_dig, next_carry, act_id, inv_id) => {
                        match multi_digit_add_loop(
                            radix.clone(),
                            (place.clone()).saturating_add(1u64),
                            tail.clone(),
                            next_carry.clone(),
                        ) {
                            ArithmeticOutcome::MultiDigitResult(rest_digits, final_carry) => {
                                ArithmeticOutcome::MultiDigitResult(
                                    {
                                        let mut __v = vec![out_dig.clone()];
                                        __v.extend(rest_digits.clone());
                                        __v
                                    },
                                    final_carry.clone(),
                                )
                            }
                            ArithmeticOutcome::PlaceStep(a, b, c, d) => {
                                ArithmeticOutcome::InvalidRadix
                            }
                            ArithmeticOutcome::InvalidRadix => ArithmeticOutcome::InvalidRadix,
                            ArithmeticOutcome::DigitOutOfRange => {
                                ArithmeticOutcome::DigitOutOfRange
                            }
                            ArithmeticOutcome::InvalidCarryTrit => {
                                ArithmeticOutcome::InvalidCarryTrit
                            }
                            ArithmeticOutcome::PlaceLimitExceeded => {
                                ArithmeticOutcome::PlaceLimitExceeded
                            }
                            ArithmeticOutcome::AblationMutedMemory => {
                                ArithmeticOutcome::AblationMutedMemory
                            }
                        }
                    }
                    ArithmeticOutcome::MultiDigitResult(ds, fc) => ArithmeticOutcome::InvalidRadix,
                    ArithmeticOutcome::InvalidRadix => ArithmeticOutcome::InvalidRadix,
                    ArithmeticOutcome::DigitOutOfRange => ArithmeticOutcome::DigitOutOfRange,
                    ArithmeticOutcome::InvalidCarryTrit => ArithmeticOutcome::InvalidCarryTrit,
                    ArithmeticOutcome::PlaceLimitExceeded => ArithmeticOutcome::PlaceLimitExceeded,
                    ArithmeticOutcome::AblationMutedMemory => {
                        ArithmeticOutcome::AblationMutedMemory
                    }
                }
            }
        }
    }

    /// Function `R4.ExactArithmetic.multiDigitRadixAdd`.
    pub fn multi_digit_radix_add(radix: u64, pairs: Vec<DigitPair>) -> ArithmeticOutcome {
        if (!(radix_is_valid(radix.clone()))) {
            ArithmeticOutcome::InvalidRadix
        } else {
            multi_digit_add_loop(radix.clone(), 0u64, pairs.clone(), 0u64)
        }
    }

    /// Function `R4.ExactArithmetic.prllPotential`.
    pub fn prll_potential(score: u64, min_score: u64) -> u64 {
        (subtract_nat(score.clone(), min_score.clone())).saturating_add(1u64)
    }

    /// Function `R4.ExactArithmetic.divRemLoop`.
    pub fn div_rem_loop(rem: u64, denom: u64, quot: u64, budget: u64) -> LikelihoodOutcome {
        {
            let __scrut = budget.clone();
            if __scrut == 0 {
                LikelihoodOutcome::MassAllocated((1u64).saturating_add(quot.clone()), rem.clone())
            } else {
                let rest_budget = __scrut - 1;
                if ((denom.clone()) <= (rem.clone())) {
                    div_rem_loop(
                        subtract_nat(rem.clone(), denom.clone()),
                        denom.clone(),
                        (quot.clone()).saturating_add(1u64),
                        rest_budget.clone(),
                    )
                } else {
                    LikelihoodOutcome::MassAllocated(
                        (1u64).saturating_add(quot.clone()),
                        rem.clone(),
                    )
                }
            }
        }
    }

    /// Function `R4.ExactArithmetic.prllAllocateMass`.
    pub fn prll_allocate_mass(
        potential: u64,
        total_potential: u64,
        remaining_pool: u64,
    ) -> LikelihoodOutcome {
        if ((total_potential.clone()) == (0u64)) {
            LikelihoodOutcome::RefusedEmptyScores
        } else {
            {
                let scaled: u64 = scale_nat(potential.clone(), remaining_pool.clone());
                div_rem_loop(
                    scaled.clone(),
                    total_potential.clone(),
                    0u64,
                    scaled.clone(),
                )
            }
        }
    }

    /// Function `R4.ExactArithmetic.bitsPerByteFraction`.
    pub fn bits_per_byte_fraction(
        total_bits_q12: u64,
        byte_count: u64,
        target_mass: u64,
    ) -> LikelihoodOutcome {
        if ((target_mass.clone()) == (0u64)) {
            LikelihoodOutcome::RefusedMassZero
        } else {
            if ((byte_count.clone()) == (0u64)) {
                LikelihoodOutcome::RefusedZeroBytes
            } else {
                LikelihoodOutcome::BpbFraction(
                    total_bits_q12.clone(),
                    scale_nat(byte_count.clone(), 4096u64),
                )
            }
        }
    }

    /// Function `R4.ExactArithmetic.admitNafEnvelope`.
    pub fn admit_naf_envelope(
        magic_ok: bool,
        domain_code: u64,
        byte_len: u64,
        high_byte: u64,
        value: u64,
        max_bytes: u64,
        policy_max_val: u64,
    ) -> NafOutcome {
        if (!(magic_ok.clone())) {
            NafOutcome::InvalidBadMagic
        } else {
            if ((max_bytes.clone()) < (byte_len.clone())) {
                NafOutcome::ResourceRefusal
            } else {
                if ((domain_code.clone()) == (4u64)) {
                    NafOutcome::Unresolved
                } else {
                    if ((domain_code.clone()) == (3u64)) {
                        NafOutcome::UnsupportedDomain
                    } else {
                        if (((1u64) < (byte_len.clone())) && ((high_byte.clone()) == (0u64))) {
                            NafOutcome::InvalidNonMinimalUvar
                        } else {
                            if ((policy_max_val.clone()) < (value.clone())) {
                                NafOutcome::PolicyOutOfDomain
                            } else {
                                NafOutcome::AcceptedCanonical(value.clone(), byte_len.clone())
                            }
                        }
                    }
                }
            }
        }
    }
}
