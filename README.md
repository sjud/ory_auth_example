# Ory Auth Axum

This example will show you how to integrate with Ory Kratos, Ory Oathkeeper, and Ory Keto for a comprenesive authorization flow.
<br>This example includes a UI client that wraps Ory functionality written in Leptos as an Admin interface.
<br> We'll set up Ory Kratos/Oathkeepr/Keto services using their respective docker images. Use mailcrab as a local email server
for development purposes.

Also Including(but not limited to):<br>
<ul>
    <li>Being a SSO Provider. <li>
    <li>Social Sign In, (Kratos) </li>
    <li>Multi-factor authentication, (Kratos with Mailcrab as our email test server) </li>
    <li>Account verification, (Kratos with Mailcrab as our email test server) </li>
    <li>Account recovery, (Kratos with Mailcrab as our email test server) </li>
    <li>Profile and account management, (Kratos) </li>
    <li>Authorization Middleware on Server Functions, (We use Oathkeeper to mutate the request and parse the request in our middleware)</li>
    <li>Authenticating Users, (Oathkeeper)</li>
    <li>Roles & Permissions, (Keto) </li>
</ul>



### How logging in works
https://www.ory.sh/docs/kratos/quickstart

<ol>
<li>
On our front page we have this address. So when you click login you are directed to the Ory Kratos server. It will redirect
you to our login route after adding a csrf token.

```html
            <li><a href="http://127.0.0.1:4433/self-service/login/browser?return_to=http://127.0.0.1:3000/login">Login</a></li>
```
</li>
<li>
Our login route renders our login component and then extracts the flow_id from the url, kratos will append `?flow=<flow_id>` to our return url.

```rust
    let flow_id = create_rw_signal(String::new());
    create_effect(move|_|{
        if let Some(flow) =  use_query_map().get_untracked().get("flow") {
            flow_id.set(flow.to_string());
        } else {
            //redirect to homepage
        }
    });
```
</li>
<li>
We have a login form and we post it to Kratos

```rust
    <form action=move||{format!("http://127.0.0.1:4433/self-service/login/flows?id={}",flow_id())} method="post">
      //<input type="text" style="display:none;" id="csrf_token" value=""/>
      <label for="email">Email:</label><br/>
      <input type="text" id="email" name="email" value=""/><br/>
      <label for="password">Password:</label><br/>
      <input type="password" id="password" name="password" value=""/><br/><br/>
      <input type="submit" value="Login"/>
    </form>
```
</li>
</ol>

### Running Our Services
Use docker compose to run our services on the same network so that they can communicate with eachother via their local network.
```sh
docker compose up
```
### Our clients (DEPRECIATED)

```sh
(cd user_app && cargo leptos serve)
```
And navigate to localhost:3000 to see the user client.

```sh
(cd admin_app && cargo leptos serve)
```
And navigate to localhost:3002 to see the admin client.

### Kratos
```sh
echo "current directory: $(pwd)" && 
docker run --rm \
  --name ory-kratos \
  -p 4433:4433 -p 4434:4434 \
  -v "$(pwd)"/kratos:/etc/config/kratos \
  oryd/kratos:v1.1.0 \
  serve --config /etc/config/kratos/kratos.yaml --watch-courier
```

### Oathkeeper
```sh
docker run --rm \
  --name ory-oathkeeper \
  -p 4455:4455 -p 4456:4456 \
  -v ./oathkeeper.yaml:/etc/config/oathkeeper \
  -v /path/to/your/oathkeeper/rules:/etc/rules/oathkeeper \
  oryd/oathkeeper:v0.40.6 \
  serve --config /etc/config/oathkeeper/oathkeeper.yaml
```

### Keto
```sh
docker run --rm \
  --name ory-keto \
  -p 4466:4466 \
  -v ./keto.yaml:/etc/config/keto \
  oryd/keto:v0.12.0 \
  serve --config /etc/config/keto/keto.yaml
```

### MailHog
```sh
docker run --rm -p 1025:1025 -p 8025:8025 mailhog/mailhog \
-smtp-bind-addr 127.0.0.1:1025 \
-ui-bind-addr 127.0.0.1:8025 \
-api-bind-addr 127.0.0.1:8025
```
### MailCrab
```sh
docker run --rm \                                         
    -p 1080:1080 \   
    -p 1025:1025 \ 
    marlonb/mailcrab:latest   
```

### Have feedback on this example?
@sjud on the leptos discord for thoughts, questions, feedback etc. Thanks!