use ed25519_dalek::{Signature, VerifyingKey};



pub fn verify(public_key_bytes: &[u8;32], signature_bytes: &[u8; 64], message: &[u8]) -> bool{
    let Ok(public_key) = VerifyingKey::from_bytes(public_key_bytes)else{
        return false;
    };
    let Ok(signature) = Signature::from_bytes(signature_bytes) else{
        return false;
    };
    public_key.verify(message, &signature).is_ok()
}