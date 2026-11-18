Feature: Prism capabilities/1 Network Gate, CORS-Direct GitHub, SpiralCore Failback & Observatory .holo Build (R4-59)

  @R4-59 @build
  Scenario: R4-59 Prism capabilities/1 network gate, CORS-direct GitHub REST, unauthenticated SpiralCore failback, RFC 8484 DoH, SSRF blocking, and ?surface=observatory .holo emission
    Given SpiralCoreHostBridge configured with Prism capabilities/1 policy mode (sandboxed vs full_spiralcore), SECRET_HOST_ALLOWLIST for GITHUB_TOKEN, and an opaque ?surface=observatory iframe
    When the bridge dispatches curl, wget, nslookup, dig, host, or git clone/branch/commit/push/pr requests
    Then Sandboxed mode permits the 10 pinned hosts and rejects others with CapabilityDenied, switching to Full SpiralCore without consentGranted returns RelayConsentRequired, and private/loopback/link-local SSRF targets fail with PrivateAddressBlocked
    And unauthenticated requests in Full SpiralCore mode try direct fetch or RFC 8484 DoH first and fail back to AF_VSOCK relayWsUrl on CORS TypeError while emitting audit_event records
    And credentialed requests targeting api.github.com/github.com or carrying Authorization/GITHUB_TOKEN use CORS-direct api.github.com REST + Git Data APIs and strictly fail with RelayForbiddenForCredentialedHost on failure without touching the relay
    And ?surface=observatory hides IDE chrome, initializes 4 Holo v13 canvases and message listeners before posting uor-observatory/1 ready, syncs host CSS vars, renders live metrics + .uorc state, and xtask build-holo --emit-fingerprint-dir emits content-addressed .holo archives beside dual weight kappa pins
