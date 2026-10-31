//! Formal Route contracts; no host model execution or GPU qualification.

mod support;

#[test]
fn bounded_route_operator_r4_07() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/RouteControls.lex.tex",
        "RouteControls",
        "8faa65782e41dce741ecb15f4f409f57b0bc9c3e7e159abc036930fbac4f9dff",
        &[
            "R4.Binder",
            "R4.Carrier",
            "R4.Geometry",
            "R4.Objective",
            "R4.Route",
            "R4.Types",
            "RouteControls",
        ],
    )
}
