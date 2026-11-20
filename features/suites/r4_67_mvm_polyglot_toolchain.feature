Feature: MVM Polyglot Toolchain Completion across cargo, npm, and C99 stdlib (R4-67)

  @R4-67 @build
  Scenario: In-MVM cargo/rustc AST execution, npm test/start/build/ls/uninstall lifecycle, and C99 <stdlib.h> SYS_brk heap allocation
    Given the formal R4.MvmPolyglotToolchain module in src/R4/MvmPolyglotToolchain.lex.tex and controls in tests/lexlean/MvmPolyglotToolchainControls.lex.tex
    When evaluating MVM terminal cargo/rustc subcommands, npm lifecycle subcommands, and C99 stdlib SYS_brk heap alignment
    Then check/test/clippy/build/run/version cargo subcommands admit as CargoAstToolchainAdmitted, npm init/install/run/test/start/build/ls/uninstall/version subcommands admit as NpmSubcommandAdmitted, and 8-byte aligned SYS_brk heap allocations admit as C99StdlibHeapAdmitted
    And unsupported toolchain subcommands refuse with RefusedUnsupportedToolchainSubcommand, mutations touching protected canonical paths refuse with RefusedProtectedPathToolchainMutation, and unaligned heap breaks refuse with RefusedUnalignedHeapBreak
