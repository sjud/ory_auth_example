use super::*;

use crate::kratos_utils::*;

use ory_kratos_client::models::RegistrationFlow;
use ory_kratos_client::models::UiContainer;
use ory_kratos_client::models::UiText;
#[cfg(feature = "ssr")]
use tracing::debug;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ViewableRegistrationFlow(RegistrationFlow);
impl IntoView for ViewableRegistrationFlow {
    fn into_view(self) -> View {
        format!("{:?}", self).into_view()
    }
}
#[tracing::instrument]
#[server]
pub async fn init_registration() -> Result<ViewableRegistrationFlow, ServerFnError> {
    let client = reqwest::ClientBuilder::new()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    // Get the csrf_token cookie.
    let resp = client
        .get("http://127.0.0.1:4433/self-service/registration/browser")
        .send()
        .await?;
    let first_cookie = resp
        .cookies()
        .next()
        .ok_or(ServerFnError::new("Expecting a first cookie"))?;
    let csrf_token = first_cookie.value();
    let location = resp
        .headers()
        .get("Location")
        .ok_or(ServerFnError::new("expecting location in headers"))?
        .to_str()?;
    // Parses the url and takes first query which will be flow=FLOW_ID and we get FLOW_ID at .1
    let location_url = url::Url::parse(location)?;
    let id = location_url
        .query_pairs()
        .next()
        .ok_or(ServerFnError::new(
            "Expecting query in location header value",
        ))?
        .1;
    let set_cookie = resp
        .headers()
        .get("set-cookie")
        .ok_or(ServerFnError::new("expecting set-cookie in headers"))?
        .to_str()?;
    let resp = client
        .get("http://127.0.0.1:4433/self-service/registration/flows")
        .query(&[("id", id)])
        .header("x-csrf-token", csrf_token)
        .send()
        .await?;
    debug!("{:#?}", resp);
    let flow = resp.json::<ViewableRegistrationFlow>().await?;
    let opts = expect_context::<leptos_axum::ResponseOptions>();
    opts.insert_header(
        axum::http::HeaderName::from_static("cache-control"),
        axum::http::HeaderValue::from_str("private, no-cache, no-store, must-revalidate")?,
    );
    opts.insert_header(
        axum::http::HeaderName::from_static("set-cookie"),
        axum::http::HeaderValue::from_str(set_cookie)?,
    );
    debug!("{:#?}", flow);
    Ok(flow)
}

#[tracing::instrument]
#[server]
pub async fn register(
    body: HashMap<String, String>,
) -> Result<Option<ViewableRegistrationFlow>, ServerFnError> {
    let mut body = body;
    let action = body
        .remove("action")
        .ok_or(ServerFnError::new("Can't find action on body."))?;
    let cookie_jar = leptos_axum::extract::<axum_extra::extract::CookieJar>().await?;
    let csrf_cookie = cookie_jar
        .iter()
        .filter(|cookie| cookie.name().contains("csrf_token"))
        .next()
        .ok_or(ServerFnError::new(
            "Expecting a csrf_token cookie to already be set if fetching a pre-existing flow",
        ))?;
    let csrf_token = csrf_cookie.value();
    let client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let resp = client
        .post(action)
        .header("x-csrf-token", csrf_token)
        .header("content-type", "application/json")
        .header(
            "cookie",
            format!("{}={}", csrf_cookie.name(), csrf_cookie.value()),
        )
        .body(serde_json::to_string(&body)?)
        .send()
        .await?;
    debug!("{:#?}", resp);

    let opts = expect_context::<leptos_axum::ResponseOptions>();
    opts.insert_header(
        axum::http::HeaderName::from_static("cache-control"),
        axum::http::HeaderValue::from_str("private, no-cache, no-store, must-revalidate")?,
    );
    let mut is_done = false;
    for value in resp.headers().get_all("set-cookie").iter() {
        if value.to_str()?.contains("ory_kratos_session") {
            is_done = true;
        }
        opts.insert_header(
            axum::http::HeaderName::from_static("set-cookie"),
            axum::http::HeaderValue::from_str(value.to_str()?)?,
        );
    }

    if resp.status().as_u16() == 403 {
        Err(ServerFnError::new(resp.text().await?))
    } else if is_done {
        debug!("done");
        Ok(None)
    } else {
        let flow = resp.json::<ViewableRegistrationFlow>().await?;
        debug!("{:#?}", flow);
        Ok(Some(flow))
    }
}

#[component]
pub fn RegistrationPage() -> impl IntoView {
    let register = Action::<Register, _>::server();

    // when we hit the page initiate a flow with kratos and get back data for ui renering.
    let registration_flow =
        create_local_resource(|| (), |_| async move { init_registration().await });
    // Is none if user hasn't submitted data.
    let register_resp =
        create_rw_signal(None::<Result<Option<ViewableRegistrationFlow>, ServerFnError>>);
    // after user tries to register we update the signal resp.
    create_effect(move |_| {
        if let Some(resp) = register.value().get() {
            register_resp.set(Some(resp))
        }
    });
    // Merge our resource and our action results into a single signal.
    // if the user hasn't tried to registet yet we'll render the initial flow.
    // if they have, we'll render the updated flow (including error messages etc).
    let registration_flow = Signal::derive(move || {
        if let Some(flow) = register_resp.get() {
            Some(flow)
        } else {
            registration_flow
                .get()
                .map(|inner| inner.map(|inner| Some(inner)))
        }
    });
    // this is the body of our registration form, we don't know what the inputs are so it's a stand in for some
    // json map of unknown argument length with type of string.
    let body = create_rw_signal(HashMap::new());
    view! {
        // we'll render the fallback when the user hits the page for the first time
      <Suspense fallback=||view!{Loading Login Details}>
        // if we get any errors, from either server functions we've merged we'll render them here.
        <ErrorBoundary fallback=|errors|format!("ERRORS: {:?}",errors.get()).into_view()>
        {
          move ||
          // this is the resource XOR the results of the register action.
          registration_flow.get().map(|resp|{
                match resp {
                    // TODO add Oauth using the flow args (see type docs)
                    Ok(Some(ViewableRegistrationFlow(RegistrationFlow{ui:box UiContainer{nodes,action,messages,..},..}))) => {
                            let form_inner_html = nodes.into_iter().map(|node|node_html(node,body)).collect_view();
                            // tells our intermediary server function where to pass on the data to.
                            body.update(|map|{_=map.insert(String::from("action"),action);});

                            view!{
                                <form on:submit=move|e|{
                                    e.prevent_default();
                                    register.dispatch(Register{body:body.get()});
                                }>
                                {form_inner_html}
                                // node_html renders messages for each node and these are the messages attached to the entire form.
                                {messages.map(|messages|{
                                    view!{
                                        <For
                                            each=move || messages.clone().into_iter()
                                            key=|text| text.id
                                            children=move |text: UiText| {
                                              view! {
                                                <p id=text.id>{text.text}</p>
                                              }
                                            }
                                        />
                                    }
                                }).unwrap_or_default()}
                                </form>
                            }.into_view()

                    },
                    Ok(None) => {
                        view!{<Redirect path="/verification"/>}.into_view()
                    }
                    err => err.into_view(),
                }
            })
          }
        </ErrorBoundary>
      </Suspense>
    }
}

/*
#[tracing::instrument]
#[server]
pub async fn fetch_preexisting_registration_flow(flow_id: String) -> Result<ViewableRegistrationFlow, ServerFnError> {
    let cookie_jar = leptos_axum::extract::<axum_extra::extract::CookieJar>().await?;
    // because csrf_token has a lot of nonsense attached to it's name.
    // i.e csrf_token8912490183409829084029384092....
    let csrf_cookie = cookie_jar
        .iter()
        .filter(|cookie| cookie.name().contains("csrf_token"))
        .next()
        .ok_or(ServerFnError::new(
            "Expecting a csrf_token cookie to already be set if fetching a pre-existing flow",
        ))?;
    let csrf_token = csrf_cookie.value();
    let client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let flow = client
        .get("http://127.0.0.1:4433/self-service/registration/flows")
        .query(&[("id", flow_id)])
        .header("x-csrf-token", csrf_token)
        .header(
            "cookie",
            format!("{}={}", csrf_cookie.name(), csrf_cookie.value()),
        )
        .send()
        .await?
        .json::<ViewableRegistrationFlow>()
        .await?;
    Ok(flow)
}
 */
