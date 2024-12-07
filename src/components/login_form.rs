use leptos::prelude::*;

#[component]
pub fn LoginForm(
    name: String,
    logo: String,
    form_value: RwSignal<Option<Result<String, ServerFnError>>>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="flex min-h-full flex-col justify-center px-6 py-12 lg:px-8">
            <LoginFormLogo name=name logo=logo/>
            <div class="mt-10 sm:mx-auto sm:w-full sm:max-w-sm">
                {children()} <p>{form_value}</p>
            </div>
        </div>
    }
}

#[component]
pub fn LoginFormLogo(name: String, logo: String) -> impl IntoView {
    view! {
        <div class="sm:mx-auto sm:w-full sm:max-w-sm">
            <img class="mx-auto h-20 w-auto" src=logo alt=name.clone()/>
            <h2 class="mt-10 text-center text-2xl font-bold leading-9 tracking-tight text-gray-900">
                {format!("Sign in to your {name} account")}
            </h2>
        </div>
    }
}
