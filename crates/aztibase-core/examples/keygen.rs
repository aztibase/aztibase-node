use aztibase_core::{BlsKeypair, Keypair, address_from_pubkey};

fn main() {
    let ed = Keypair::generate();
    let bls = BlsKeypair::generate();

    let pubkey = ed.public_key();
    let pubkey_bytes = pubkey.as_bytes();
    let address = address_from_pubkey(pubkey_bytes);

    println!("{{");
    println!("  \"public_key\": \"{}\",", hex::encode(pubkey_bytes));
    println!("  \"secret_key\": \"{}\",", hex::encode(ed.secret_bytes()));
    println!("  \"address\": \"{}\",", hex::encode(address));
    println!(
        "  \"bls_public_key\": \"{}\",",
        hex::encode(bls.public_key().as_bytes())
    );
    println!(
        "  \"bls_secret_key\": \"{}\"",
        hex::encode(bls.secret_bytes())
    );
    println!("}}");
}
