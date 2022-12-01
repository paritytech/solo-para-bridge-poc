// Example of signing, then verifying in a separate context, with the use case of using the node's
// keys as auth for a separate service in mind

fn sign() {
	// Any message that needs to be signed and verified
	let auth_data = b"auth data";
	log::info!(target: "runtime::client", "Signing authentication data...");
	runtime_interface.sender().sign(example_auth_data)

	// Then later, when verifying in a *separate* context:
	// use primitives::shared::Signature;
	//      After receiving public key, message, and signature in auth request...
	//     if pub_key.verify(example_auth_data, &Signature::from(auth_data_signature)) {
	//          log::info!("verified");
	//     } else {
	//          log::info!("not verified");
	//     }
}
