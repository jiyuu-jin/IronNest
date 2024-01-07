use {crate::integrations::iron_nest::types::Device, leptos::*};

#[server(GetDevices)]
pub async fn get_devices() -> Result<Vec<Device>, ServerFnError> {
    use {
        crate::integrations::iron_nest::types::Device,
        sqlx::{Pool, Sqlite},
        std::sync::Arc,
    };

    let pool = use_context::<Arc<Pool<Sqlite>>>().unwrap();

    let query = "
        SELECT id, name, device_type, ip, power_state, 0 AS battery_percentage 
        FROM devices
    ";
    sqlx::query_as::<Sqlite, Device>(query)
        .fetch_all(&*pool)
        .await
        .map_err(Into::into)
}

#[component]
pub fn DevicesPage() -> impl IntoView {
    let devices = create_resource(|| (), |_| get_devices());
    view! {
        <main class="lg:pl-20">
            <div class="lg:pl-4 -mx-4 -my-2 overflow-x-auto sm:-mx-6 lg:-mx-8">
                <div class="inline-block min-w-full py-2 align-middle sm:px-6 lg:px-8">
                    <table class="w-full divide-y divide-gray-300">
                        <thead>
                            <tr>
                                <th
                                    scope="col"
                                    class="py-3 pl-4 pr-3 text-left text-xs font-medium uppercase tracking-wide text-gray-500 sm:pl-0"
                                >
                                    Name
                                </th>
                                <th
                                    scope="col"
                                    class="px-3 py-3 text-left text-xs font-medium uppercase tracking-wide text-gray-500"
                                >
                                    "Device Type"
                                </th>
                                <th
                                    scope="col"
                                    class="px-3 py-3 text-left text-xs font-medium uppercase tracking-wide text-gray-500"
                                >
                                    "IP"
                                </th>
                                <th
                                    scope="col"
                                    class="px-3 py-3 text-left text-xs font-medium uppercase tracking-wide text-gray-500"
                                >
                                    "Power State"
                                </th>
                                <th scope="col" class="relative py-3 pl-3 pr-4 sm:pr-0">
                                    <span class="sr-only">Edit</span>
                                </th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-gray-200 bg-white">
                            <Suspense fallback=|| {
                                view! {
                                    <tr>
                                        <td>"Loading devices..."</td>
                                    </tr>
                                }
                            }>
                                {move || {
                                    devices
                                        .get()
                                        .map(|data| {
                                            data.map(|data| {
                                                data.iter()
                                                    .map(|device| {
                                                        view! {
                                                            <tr>
                                                                <td class="whitespace-nowrap py-4 pl-4 pr-3 text-sm font-medium text-gray-900 sm:pl-0">
                                                                    {&device.name}
                                                                </td>
                                                                <th
                                                                    scope="col"
                                                                    class="px-3 py-3 text-left text-xs font-medium uppercase tracking-wide text-gray-500"
                                                                >
                                                                    {device.device_type.to_string()}
                                                                </th>
                                                                <th
                                                                    scope="col"
                                                                    class="px-3 py-3 text-left text-xs font-medium uppercase tracking-wide text-gray-500"
                                                                >
                                                                    {&device.ip}
                                                                </th>
                                                                <th
                                                                    scope="col"
                                                                    class="px-3 py-3 text-left text-xs font-medium uppercase tracking-wide text-gray-500"
                                                                >
                                                                    {device.power_state}
                                                                </th>
                                                            </tr>
                                                        }
                                                    })
                                                    .collect::<Vec<_>>()
                                            })
                                        })
                                }}

                            </Suspense>
                        </tbody>
                    </table>
                </div>
            </div>
        </main>
    }
}
