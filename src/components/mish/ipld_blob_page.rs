use {
    crate::{
        components::{
            layout::{Toast, ToastContext},
            mish::{json_editor::JsonEditor, text_editor::TextEditor},
        },
        ipld_codecs,
    },
    cid::Cid,
    ipld_core::codec::{Codec, Links},
    leptos::prelude::*,
    leptos_router::{
        hooks::{use_navigate, use_params},
        params::Params,
    },
    serde::{Deserialize, Serialize},
    serde_ipld_dagjson::codec::DagJsonCodec,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MishState {
    name: String,
    state: serde_json::Value,
}

#[cfg(feature = "ssr")]
#[derive(sqlx::FromRow, Serialize, Deserialize, Clone, Debug)]
pub struct MishStateRow {
    name: String,
    state: serde_json::Value,
}

#[server(GetIpldBlob)]
async fn get_ipld_blob(cid: String) -> Result<Option<Vec<u8>>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>().unwrap();
    let ipld_blob = get_ipld_blob_query(&pool, &cid.parse::<Cid>().unwrap()).await?;
    Ok(ipld_blob)
}

#[cfg(feature = "ssr")]
pub async fn get_ipld_blob_query(
    pool: &sqlx::PgPool,
    cid: &Cid,
) -> Result<Option<Vec<u8>>, sqlx::Error> {
    #[derive(sqlx::FromRow, Serialize, Deserialize, Clone, Debug)]
    struct Row {
        content: Vec<u8>,
    }
    let query = "
        SELECT content
        FROM ipld_blobs
        WHERE cid = $1
    ";
    sqlx::query_as::<_, Row>(query)
        .bind(cid.to_bytes())
        .fetch_optional(pool)
        .await
        .map(|row| row.map(|row| row.content))
}

#[server(SetIpldBlob)]
async fn set_ipld_blob(content: String) -> Result<String, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>().unwrap();
    let content = hex::decode(content).unwrap();
    let cid = set_mish_state_query(&pool, content).await?;
    println!("cid: {}", cid);
    Ok(cid.to_string())
}

#[cfg(feature = "ssr")]
pub async fn set_mish_state_query(
    pool: &sqlx::PgPool,
    content: Vec<u8>,
) -> Result<Cid, sqlx::Error> {
    use multihash_codetable::{Code, MultihashDigest};
    // TODO support dag-json and raw?
    let cid = Cid::new_v1(ipld_codecs::RAW, Code::Sha2_256.digest(&content));
    let query = "
        INSERT INTO ipld_blobs (cid, content)
        VALUES ($1, $2)
        ON CONFLICT (cid) DO NOTHING
    ";
    sqlx::query(query)
        .bind(cid.to_bytes())
        .bind(content)
        .execute(pool)
        .await
        .unwrap();
    Ok(cid)
}

#[component]
pub fn IpldBlobPage() -> impl IntoView {
    #[derive(Params, PartialEq)]
    struct IpldBlobParams {
        cid: Option<String>,
    }
    let params = use_params::<IpldBlobParams>();
    let cid = move || {
        params
            .read()
            .as_ref()
            .unwrap()
            .cid
            .clone()
            .unwrap()
            .parse::<Cid>()
            .unwrap()
    };

    let values = Resource::new(
        cid, // TODO add action value as input, and so we can use this value immediately instead of calling get_mish_state
        |cid| get_ipld_blob(cid.to_string()),
    );

    let set_ipld_blob_action = ServerAction::<SetIpldBlob>::new();
    Effect::new(move || {
        let new_cid = set_ipld_blob_action.value().get();
        if let Some(Ok(new_cid)) = new_cid {
            let navigate = use_navigate();
            navigate(
                &format!("/settings/dag-inspector/ipld-blob/{new_cid}"),
                Default::default(),
            );
        }
    });

    let toast = use_context::<ToastContext>().unwrap();
    Resource::new(
        move || {
            (
                set_ipld_blob_action.value().get(),
                set_ipld_blob_action.version().get(),
            )
        },
        move |(value, _version)| async move {
            if matches!(value, Some(Ok(_))) {
                toast.set(Some(Toast("Mish State saved".to_owned())));
            }
        },
    );

    let raw_editor_mode = RwSignal::new(false);

    view! {
        <main class="lg:p-40 lg:pt-20 cursor-pointer">
            <div class="mx-auto max-w-2xl space-y-16 sm:space-y-20 lg:mx-0 lg:max-w-none">
                <div>
                    <div>
                        <a href="/settings/dag-inspector">"Back to Dag Inspector"</a>
                    </div>
                    <div>
                        <label for="raw-editor-mode">"RAW editor mode"</label>
                        <input type="checkbox" id="raw-editor-mode" bind:checked=raw_editor_mode />
                    </div>
                    <Suspense fallback=|| {
                        view! { <p>"Loading IPLD Blob..."</p> }
                    }>
                        <div>
                            {move || {
                                values
                                    .get()
                                    .map(|values| {
                                        match values {
                                            Err(e) => {
                                                view! { <p>"Error loading IPLD Blob: " {e.to_string()}</p> }
                                                    .into_any()
                                            }
                                            Ok(value) => {
                                                value
                                                    .map(|state| {
                                                        let codec = cid().codec();
                                                        match codec {
                                                            ipld_codecs::RAW => {
                                                                view! {
                                                                    <TextEditor
                                                                        state=hex::encode(state)
                                                                        set_config_server_action=move |content| {
                                                                            set_ipld_blob_action.dispatch(SetIpldBlob { content });
                                                                        }
                                                                    />
                                                                }
                                                                    .into_any()
                                                            }
                                                            ipld_codecs::DAG_JSON => {
                                                                let parsed = <DagJsonCodec as Codec<
                                                                    serde_json::Value,
                                                                >>::decode_from_slice(&state);
                                                                match parsed {
                                                                    Ok(parsed) => {
                                                                        // content is already hex encoded
                                                                        view! {
                                                                            <JsonEditor
                                                                                state=Some(parsed)
                                                                                set_config_server_action=move |content| {
                                                                                    set_ipld_blob_action
                                                                                        .dispatch(SetIpldBlob {
                                                                                            content: hex::encode(content),
                                                                                        });
                                                                                }
                                                                            />
                                                                        }
                                                                            .into_any()
                                                                    }
                                                                    Err(e) => {
                                                                        view! { <p>"Error parsing DAG-JSON: " {e.to_string()}</p> }
                                                                            .into_any()
                                                                    }
                                                                }
                                                            }
                                                            _ => {
                                                                view! { <p>"Unsupported codec: " {codec}</p> }.into_any()
                                                            }
                                                        }
                                                    })
                                                    .unwrap_or_else(|| {
                                                        view! {
                                                            <p>"No value for this CID"</p>
                                                            <p>"CID: " {cid().to_string()}</p>
                                                        }
                                                            .into_any()
                                                    })
                                            }
                                        }
                                    })
                            }}
                        </div>
                        <div>{move || format!("{:?}", values.get())}</div>
                        <div>
                            {move || {
                                if let Some(Ok(Some(data))) = values.get() {
                                    if cid().codec() == ipld_codecs::DAG_JSON {
                                        let links = <DagJsonCodec as Links>::links(&data);
                                        match links {
                                            Ok(links) => {
                                                view! {
                                                    {links
                                                        .into_iter()
                                                        .map(|link| {
                                                            view! {
                                                                <p>
                                                                    <a href=format!(
                                                                        "/settings/dag-inspector/ipld-blob/{link}",
                                                                    )>"Link: "{link.to_string()}</a>
                                                                </p>
                                                            }
                                                                .into_any()
                                                        })
                                                        .collect::<Vec<_>>()}
                                                }
                                                    .into_any()
                                            }
                                            Err(e) => {
                                                view! { <p>{format!("Error getting links: {e}")}</p> }
                                                    .into_any()
                                            }
                                        }
                                    } else {
                                        ().into_any()
                                    }
                                } else {
                                    ().into_any()
                                }
                            }}
                        </div>
                    </Suspense>
                </div>
            </div>
        </main>
    }
}
