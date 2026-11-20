Feature: Single-Source MVM Linux Container, SpiralCore CORS Preemption & Interactive Execution (R4-63)

  @R4-63 @build
  Scenario: Single-source MVM /workspace CoW VFS, AF_INET SPVS encapsulation, unauthenticated public CORS preemption, fail-closed interactive execution precedence, and <=120-token KV state header
    Given the formal R4.MvmLinuxContainer module in src/R4/MvmLinuxContainer.lex.tex and controls in tests/lexlean/MvmLinuxContainerControls.lex.tex
    When evaluating single-source MVM workspace authority, SpiralCore AF_INET socket encapsulation, public CORS relay preemption, Agent/Plan/Auto-run command precedence, and KV state header width
    Then SingleSourceMvmCow with consistent CoW state is admitted while SplitBrainDuplicateStore refuses with RefusedSplitBrainWorkspace
    And AF_INET is encapsulated only when MvmNetworkMode is SpiralCoreEncapsulated while ZeroNicIsolated keeps RefusedZeroNicIsolated
    And public CORS blocks preempt through StatelessRelayCorsPreempted while private SSRF targets refuse with RefusedPrivateAddressSsrf and credentialed proxy attempts refuse with RefusedCredentialedCorsProxy
    And interactive commands enforce Deny over PlanReadOnly over Agent Auto-run with token counts capped at 120
