use kivractl::kivra::error::Error;
use kivractl::kivra::request;
use kivractl::kivra::qr;

fn main() {
    let resp = run();
    println!("{:?}", resp);
}

fn run() -> Result<request::AuthResponse, Error> {
    let client = request::client();
    let config = request::get_config(&client)?;
    let auth_response = request::start_auth(&client, &config)?;

    let mut status = request::check_auth(&client, &auth_response.next_poll_url)?;
    let qrcode = qr::encode(&status.qr_code)?;
    println!("{}", qrcode);
    for _ in 1..5 {
	std::thread::sleep(std::time::Duration::from_secs(status.retry_after.into()));
	status = request::check_auth(&client, &status.next_poll_url)?;
	let qrcode = qr::encode(&status.qr_code)?;
	println!("{}", qrcode);
    }
    Ok(auth_response)
}
