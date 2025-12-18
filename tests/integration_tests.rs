mod common;

use std::collections::HashMap;

use common::{response_contains, test_client, with_backend};

#[tokio::test(flavor = "current_thread")]
async fn basic_workflow() {
    with_backend(|state| async move {
        let base_url = &state.config.base_url;
        let client = test_client();

        response_contains(&client, &format!("{base_url}/"), "RubHub")
            .await
            .unwrap();

        let mut form = HashMap::new();
        let pw = "12345678901234567890";
        form.insert("username", "t");
        form.insert("email", "test@rubhub.net");
        form.insert("password", pw);

        // First we try to register with a username that's too short
        client
            .post(format!("{base_url}/registration"))
            .form(&form)
            .send()
            .await
            .expect("Registration failed")
            .error_for_status()
            .expect_err("Registration request failed");

        // Now we use the full username
        form.insert("username", "test");
        client
            .post(format!("{base_url}/registration"))
            .form(&form)
            .send()
            .await
            .expect("Registration failed")
            .error_for_status()
            .expect("Registration request failed");

        response_contains(&client, &format!("{base_url}/~test"), "test")
            .await
            .unwrap();

        client
            .get(format!("{base_url}/logout"))
            .send()
            .await
            .expect("Logout failed")
            .error_for_status()
            .expect("Logout status failed");
    })
    .await;
}
