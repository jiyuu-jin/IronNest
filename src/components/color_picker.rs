use leptos::*;

#[component]
pub fn Color_Picker(
    label: String,
    default_value: String,
    on_change: Box<dyn Fn(u8)>,
) -> impl IntoView {
    // Helper function to convert hex to saturation in HSL
    fn hex_to_saturation(hex: &str) -> Option<u8> {
        if hex.len() != 7 || !hex.starts_with('#') {
            return None;
        }

        let r = u8::from_str_radix(&hex[1..3], 16).ok()?;
        let g = u8::from_str_radix(&hex[3..5], 16).ok()?;
        let b = u8::from_str_radix(&hex[5..7], 16).ok()?;

        let r = r as f32 / 255.0;
        let g = g as f32 / 255.0;
        let b = b as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);

        let l = (max + min) / 2.0;

        let s = if max == min {
            0.0
        } else if l < 0.5 {
            (max - min) / (max + min)
        } else {
            (max - min) / (2.0 - max - min)
        };

        Some((s * 100.0).round() as u8)
    }

    view! {
        <div>
            <input
                type="color"
                id="colorPicker"
                name="colorPicker"
                value=default_value
                on:change:undelegated=move |ev| {
                    let hex_value = event_target_value(&ev);
                    if let Some(saturation) = hex_to_saturation(&hex_value) {
                        on_change(saturation);
                    } else {
                        on_change(0);
                    }
                }
            />

            <label for="colorPicker">{label}</label>
        </div>
    }
}
