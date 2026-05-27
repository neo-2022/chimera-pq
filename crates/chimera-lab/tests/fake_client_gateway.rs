use chimera_carrier::{Carrier, InMemoryCarrier};
use chimera_carrier_tls::{TlsCarrier, TlsCarrierConfig};
use chimera_client::ClientHandshake;
use chimera_gateway::GatewayHandshake;

#[test]
fn fake_client_gateway_handshake_over_in_memory_carriers() {
    let client = ClientHandshake::new_test_only([31_u8; 32]);
    let mut client_to_gateway = InMemoryCarrier::new(4096);
    let mut gateway_to_client = InMemoryCarrier::new(4096);

    assert!(client_to_gateway.send(client.client_hello_bytes()).is_ok());

    let client_hello_bytes = match client_to_gateway.recv() {
        Ok(Some(bytes)) => bytes,
        Ok(None) => unreachable!("gateway should receive client hello"),
        Err(error) => unreachable!("gateway receive should succeed: {error}"),
    };

    let gateway =
        match GatewayHandshake::accept_client_hello_bytes(&client_hello_bytes, [41_u8; 32]) {
            Ok(gateway) => gateway,
            Err(error) => unreachable!("gateway should accept client hello: {error}"),
        };
    assert!(gateway_to_client.send(gateway.server_hello_bytes()).is_ok());

    let server_hello_bytes = match gateway_to_client.recv() {
        Ok(Some(bytes)) => bytes,
        Ok(None) => unreachable!("client should receive server hello"),
        Err(error) => unreachable!("client receive should succeed: {error}"),
    };

    let client_session = match client.finish_from_server_hello_bytes(&server_hello_bytes) {
        Ok(session) => session,
        Err(error) => unreachable!("client should finish handshake: {error}"),
    };
    let gateway_session = match gateway.finish() {
        Ok(session) => session,
        Err(error) => unreachable!("gateway should finish handshake: {error}"),
    };

    assert_eq!(
        client_session
            .traffic_secrets
            .client_to_gateway
            .expose_for_tests(),
        gateway_session
            .traffic_secrets
            .client_to_gateway
            .expose_for_tests()
    );
}

#[test]
fn fake_client_gateway_x25519_handshake_over_in_memory_carriers() {
    let client = ClientHandshake::new_x25519([21_u8; 32], [7_u8; 32]);
    let mut client_to_gateway = InMemoryCarrier::new(4096);
    let mut gateway_to_client = InMemoryCarrier::new(4096);

    assert!(client_to_gateway.send(client.client_hello_bytes()).is_ok());

    let client_hello_bytes = match client_to_gateway.recv() {
        Ok(Some(bytes)) => bytes,
        Ok(None) => unreachable!("gateway should receive client hello"),
        Err(error) => unreachable!("gateway receive should succeed: {error}"),
    };

    let gateway = match GatewayHandshake::accept_x25519_client_hello_bytes(
        &client_hello_bytes,
        [22_u8; 32],
        [9_u8; 32],
    ) {
        Ok(gateway) => gateway,
        Err(error) => unreachable!("gateway should accept X25519 client hello: {error}"),
    };
    assert!(gateway_to_client.send(gateway.server_hello_bytes()).is_ok());

    let server_hello_bytes = match gateway_to_client.recv() {
        Ok(Some(bytes)) => bytes,
        Ok(None) => unreachable!("client should receive server hello"),
        Err(error) => unreachable!("client receive should succeed: {error}"),
    };

    let client_session = match client.finish_from_server_hello_bytes(&server_hello_bytes) {
        Ok(session) => session,
        Err(error) => unreachable!("client should finish X25519 handshake: {error}"),
    };
    let gateway_session = match gateway.finish() {
        Ok(session) => session,
        Err(error) => unreachable!("gateway should finish X25519 handshake: {error}"),
    };

    assert_eq!(
        client_session
            .traffic_secrets
            .client_to_gateway
            .expose_for_tests(),
        gateway_session
            .traffic_secrets
            .client_to_gateway
            .expose_for_tests()
    );
}

#[test]
fn fake_client_gateway_handshake_over_tls_carrier_bus() {
    let client = ClientHandshake::new_test_only([11_u8; 32]);
    let tls_config = TlsCarrierConfig {
        server_name: "gateway.example.org".to_string(),
        connect_addr: "lab.local:443".to_string(),
        connect_timeout_ms: 1000,
    };
    let mut client_to_gateway = match TlsCarrier::new(tls_config.clone()) {
        Ok(carrier) => carrier,
        Err(error) => unreachable!("client tls carrier should be created: {error}"),
    };
    let mut gateway_to_client = match TlsCarrier::new(tls_config) {
        Ok(carrier) => carrier,
        Err(error) => unreachable!("gateway tls carrier should be created: {error}"),
    };

    assert!(client_to_gateway.send(client.client_hello_bytes()).is_ok());

    let client_hello_bytes = match gateway_to_client.recv() {
        Ok(Some(bytes)) => bytes,
        Ok(None) => unreachable!("gateway should receive client hello"),
        Err(error) => unreachable!("gateway receive should succeed: {error}"),
    };

    let gateway =
        match GatewayHandshake::accept_client_hello_bytes(&client_hello_bytes, [12_u8; 32]) {
            Ok(gateway) => gateway,
            Err(error) => unreachable!("gateway should accept client hello: {error}"),
        };
    assert!(gateway_to_client.send(gateway.server_hello_bytes()).is_ok());

    let server_hello_bytes = match client_to_gateway.recv() {
        Ok(Some(bytes)) => bytes,
        Ok(None) => unreachable!("client should receive server hello"),
        Err(error) => unreachable!("client receive should succeed: {error}"),
    };

    let client_session = match client.finish_from_server_hello_bytes(&server_hello_bytes) {
        Ok(session) => session,
        Err(error) => unreachable!("client should finish handshake: {error}"),
    };
    let gateway_session = match gateway.finish() {
        Ok(session) => session,
        Err(error) => unreachable!("gateway should finish handshake: {error}"),
    };

    assert_eq!(
        client_session
            .traffic_secrets
            .client_to_gateway
            .expose_for_tests(),
        gateway_session
            .traffic_secrets
            .client_to_gateway
            .expose_for_tests()
    );
}
