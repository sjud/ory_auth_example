use super::*;

use crate::kratos_utils::*;

use ory_kratos_client::models::LoginFlow;
use ory_kratos_client::models::UiContainer;
use ory_kratos_client::models::UiText;
#[cfg(feature = "ssr")]
use tracing::debug;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ViewableLoginFlow(LoginFlow);
impl IntoView for ViewableLoginFlow {
    fn into_view(self) -> View {
        format!("{:?}", self).into_view()
    }
}
#[tracing::instrument]
#[server]
pub async fn init_login() -> Result<ViewableLoginFlow, ServerFnError> {
    let client = reqwest::ClientBuilder::new()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    // Get the csrf_token cookie.
    let resp = client
        .get("http://127.0.0.1:4433/self-service/login/browser")
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
    let flow = client
        .get("http://127.0.0.1:4433/self-service/login/flows")
        .query(&[("id", id)])
        .header("x-csrf-token", csrf_token)
        .send()
        .await?
        .json::<ViewableLoginFlow>()
        .await?;
    let opts = expect_context::<leptos_axum::ResponseOptions>();
    opts.insert_header(
        axum::http::HeaderName::from_static("set-cookie"),
        axum::http::HeaderValue::from_str(set_cookie)?,
    );
    Ok(flow)
}

#[tracing::instrument]
#[server]
pub async fn fetch_preexisting_login_flow(
    flow_id: String,
) -> Result<ViewableLoginFlow, ServerFnError> {
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
        .get("http://127.0.0.1:4433/self-service/login/flows")
        .query(&[("id", flow_id)])
        .header("x-csrf-token", csrf_token)
        .header(
            "cookie",
            format!("{}={}", csrf_cookie.name(), csrf_cookie.value()),
        )
        .send()
        .await?
        .json::<ViewableLoginFlow>()
        .await?;
    debug!("{:#?}", flow);
    Ok(flow)
}

#[component]
pub fn LoginPage() -> impl IntoView {
    let login_flow = create_resource(
        || {
            if let Some(flow_id) = use_query_map().get_untracked().get("flow").cloned() {
                FlowStages::HandleFlow(flow_id)
            } else {
                FlowStages::Init
            }
        },
        |variant| async move {
            match variant {
                FlowStages::HandleFlow(flow_id) => fetch_preexisting_login_flow(flow_id).await,
                FlowStages::Init => init_login().await,
            }
        },
    );
    let body = create_rw_signal(HashMap::new());
    view! {
      <Suspense fallback=||view!{Loading Login Details}>
        <ErrorBoundary fallback=|errors|format!("ERRORS: {:?}",errors.get()).into_view()>
        {
          move ||
            login_flow.get().map(|resp|{
                match resp {
                    // TODO add Oauth using the flow args (see type docs)
                    Ok(ViewableLoginFlow(LoginFlow{ui:box UiContainer{nodes,action,method,messages},..})) => {
                        let form_inner_html = nodes.into_iter().map(|node|node_html(node,body)).collect_view();
                        view!{
                            <form method=method action=action>
                            {form_inner_html}
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
                    err => err.into_view(),
                }
            })
          }
        </ErrorBoundary>
      </Suspense>
    }
}
