//! Formal Route contracts; no host model execution or GPU qualification.

mod support;

#[test]
fn bounded_route_operator_r4_07() -> Result<(), Box<dyn std::error::Error>> {
    support::verify_controls(
        "tests/lexlean/RouteControls.lex.tex",
        "RouteControls",
        "cb6125350b75d06db18b28d881fb23f34387ae636c0026bb69d0aecf341cc0ef",
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
