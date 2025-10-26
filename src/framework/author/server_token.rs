use md5;

pub const PROJECT_NAME: &str = "pantyhose_server";

pub fn generate_token(author_key: &str) -> String {
    let input = format!("{}{}{}", author_key, PROJECT_NAME, author_key);
    let digest = md5::compute(input);
    format!("{:x}", digest)
}

pub fn server_token_authentication(token: &str, author_key: &str) -> bool {
    let generated_token = generate_token(author_key);
    generated_token == token
}