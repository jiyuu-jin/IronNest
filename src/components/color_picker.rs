use leptos::*;

#[component]
pub fn Color_Picker(label: String, default_value: String) -> impl IntoView {
    view! {
      <div>
        <input type="color" id="colorPicker" name="colorPicker" value=default_value />
        <label for="colorPicker">{label}</label>
      </div>
    }
}
