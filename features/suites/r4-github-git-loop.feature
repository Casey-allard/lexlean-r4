Feature: Arbitrary-Repo GitHub Connect, Branch, Commit, PR, CI-Watch, and Refresh Loop (R4-40)

  @R4-40 @build
  Scenario: GitHub git loop enforces allowlisted hosts, zero guest secret leakage, valid slugs/refs, and CI/deploy gates
    Given the formal LexLean module R4.GithubAgentLoop and workflow ecosystem checks
    When a guest requests clone, read_blob, create_branch, commit_push, create_pr, pr_status, merge_pr, or watch_deploy
    Then allowlisted github.com/api.github.com HTTPS/WSS actions without guest secret leakage are admitted
    And deny policies, guest RAM secrets, non-allowlisted hosts, invalid slugs or refs, and failed CI/deploy gates are refused
    And ecosystem CI workflows automerge.yml, deploy-watch.yml, and pages.yml satisfy pinned security pins and artifact staging
