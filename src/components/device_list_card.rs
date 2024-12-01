use {
    crate::integrations::iron_nest::types::{Device, DeviceType},
    leptos::*,
};

#[component]
pub fn DeviceListCard(device: Device, children: Children) -> impl IntoView {
    let icon = match device.device_type {
        DeviceType::SmartPlug => view! {
            <svg
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                stroke-width="1.5"
                stroke="currentColor"
                class="w-6 h-6"
            >
                <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    d="m3.75 13.5 10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75Z"
                ></path>
            </svg>
        },
        DeviceType::SmartLight => view! {
            <svg
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                stroke-width="1.5"
                stroke="currentColor"
                class="w-6 h-6"
            >
                <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    d="M12 18v-5.25m0 0a6.01 6.01 0 0 0 1.5-.189m-1.5.189a6.01 6.01 0 0 1-1.5-.189m3.75 7.478a12.06 12.06 0 0 1-4.5 0m3.75 2.383a14.406 14.406 0 0 1-3 0M14.25 18v-.192c0-.983.658-1.823 1.508-2.316a7.5 7.5 0 1 0-7.517 0c.85.493 1.509 1.333 1.509 2.316V18"
                ></path>
            </svg>
        },
        DeviceType::SmartDimmer => view! {
            <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 448 512"
                stroke="currentColor"
                class="w-6 h-6"
            >
                <path d="M64 80c-8.8 0-16 7.2-16 16l0 320c0 8.8 7.2 16 16 16l320 0c8.8 0 16-7.2 16-16l0-320c0-8.8-7.2-16-16-16L64 80zM0 96C0 60.7 28.7 32 64 32l320 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 96zM152 232l144 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-144 0c-13.3 0-24-10.7-24-24s10.7-24 24-24z"></path>
            </svg>
        },
        DeviceType::SmartPowerStrip => view! {
            <svg
                fill="#000000"
                version="1.1"
                id="Capa_1"
                xmlns="http://www.w3.org/2000/svg"
                xmlns:xlink="http://www.w3.org/1999/xlink"
                viewBox="0 0 367.732 367.732"
                xml:space="preserve"
                stroke="currentColor"
                class="w-6 h-6"
            >
                <g>
                    <path d="M322.542,132.8c-0.613,0-1.204,0.093-1.761,0.264v-19.562c0-12.469-10.145-22.613-22.613-22.613H28.132
                    C12.62,90.888,0,103.508,0,119.02v48.369c0,15.512,12.62,28.132,28.132,28.132h270.036c12.469,0,22.613-10.145,22.613-22.613
                    v-28.372c0.557,0.171,1.148,0.264,1.761,0.264c18.301,0,33.19,14.889,33.19,33.191v10.663c0,18.301-14.89,33.19-33.19,33.19
                    h-73.126c-15.163,0-27.5,12.336-27.5,27.5s12.337,27.5,27.5,27.5h30.935c3.313,0,6-2.687,6-6c0-3.314-2.687-6-6-6h-30.935
                    c-8.547,0-15.5-6.953-15.5-15.5s6.953-15.5,15.5-15.5h73.126c24.918,0,45.19-20.272,45.19-45.19v-10.663
                    C367.732,153.072,347.46,132.8,322.542,132.8z M308.781,172.908c0,5.852-4.761,10.613-10.613,10.613H28.132
                    c-8.896,0-16.132-7.237-16.132-16.132V119.02c0-8.895,7.236-16.132,16.132-16.132h270.036c5.853,0,10.613,4.761,10.613,10.613
                    V172.908z"></path>
                    <path d="M291.328,115.25h-21.954c-3.313,0-6,2.687-6,6v43.909c0,3.313,2.687,6,6,6h21.954c3.313,0,6-2.687,6-6V121.25
                    C297.328,117.937,294.642,115.25,291.328,115.25z M285.328,127.25v9.955h-9.954v-9.955H285.328z M275.374,159.159v-9.955h9.954
                    v9.955H275.374z"></path>
                    <path d="M54.253,116.949c-14.478,0-26.256,11.778-26.256,26.256s11.778,26.256,26.256,26.256s26.256-11.778,26.256-26.256
                    S68.73,116.949,54.253,116.949z M54.253,157.46c-7.86,0-14.256-6.395-14.256-14.256s6.396-14.256,14.256-14.256
                    s14.256,6.395,14.256,14.256S62.113,157.46,54.253,157.46z"></path>
                    <path d="M112.141,116.949c-14.478,0-26.256,11.778-26.256,26.256s11.778,26.256,26.256,26.256s26.256-11.778,26.256-26.256
                    S126.618,116.949,112.141,116.949z M112.141,157.46c-7.86,0-14.256-6.395-14.256-14.256s6.396-14.256,14.256-14.256
                    s14.256,6.395,14.256,14.256S120.001,157.46,112.141,157.46z"></path>
                    <path d="M170.028,116.949c-14.478,0-26.256,11.778-26.256,26.256s11.778,26.256,26.256,26.256s26.256-11.778,26.256-26.256
                    S184.506,116.949,170.028,116.949z M170.028,157.46c-7.86,0-14.256-6.395-14.256-14.256s6.396-14.256,14.256-14.256
                    s14.256,6.395,14.256,14.256S177.889,157.46,170.028,157.46z"></path>
                    <path d="M227.916,116.949c-14.478,0-26.256,11.778-26.256,26.256s11.778,26.256,26.256,26.256s26.256-11.778,26.256-26.256
                    S242.394,116.949,227.916,116.949z M227.916,157.46c-7.86,0-14.256-6.395-14.256-14.256s6.396-14.256,14.256-14.256
                    s14.256,6.395,14.256,14.256S235.776,157.46,227.916,157.46z"></path>
                    <circle cx="54.253" cy="136.705" r="3.25"></circle>
                    <circle cx="54.253" cy="149.705" r="3.25"></circle>
                    <circle cx="112.141" cy="136.705" r="3.25"></circle>
                    <circle cx="112.141" cy="149.705" r="3.25"></circle>
                    <circle cx="170.029" cy="136.705" r="3.25"></circle>
                    <circle cx="170.029" cy="149.705" r="3.25"></circle>
                    <circle cx="227.917" cy="136.705" r="3.25"></circle>
                    <circle cx="227.917" cy="149.705" r="3.25"></circle>
                </g>
            </svg>
        },
        DeviceType::RingDoorbell => view! {
            <svg
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                stroke-width="1.5"
                stroke="currentColor"
                class="w-6 h-6"
            >
                <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    d="M14.857 17.082a23.848 23.848 0 0 0 5.454-1.31A8.967 8.967 0 0 1 18 9.75V9A6 6 0 0 0 6 9v.75a8.967 8.967 0 0 1-2.312 6.022c1.733.64 3.56 1.085 5.455 1.31m5.714 0a24.255 24.255 0 0 1-5.714 0m5.714 0a3 3 0 1 1-5.714 0"
                ></path>
            </svg>
        },
        DeviceType::RokuTv => view! {
            <svg
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                stroke-width="1.5"
                stroke="currentColor"
                class="w-6 h-6"
            >
                <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    d="M6 20.25h12m-7.5-3v3m3-3v3m-10.125-3h17.25c.621 0 1.125-.504 1.125-1.125V4.875c0-.621-.504-1.125-1.125-1.125H3.375c-.621 0-1.125.504-1.125 1.125v11.25c0 .621.504 1.125 1.125 1.125Z"
                ></path>
            </svg>
        },
        DeviceType::Stoplight => view! {
            <svg
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="none"
                xmlns="http://www.w3.org/2000/svg"
            >
                <path d="M15 9H9V15H15V9Z" fill="currentColor"></path>
                <path
                    fill-rule="evenodd"
                    clip-rule="evenodd"
                    d="M23 12C23 18.0751 18.0751 23 12 23C5.92487 23 1 18.0751 1 12C1 5.92487 5.92487 1 12 1C18.0751 1 23 5.92487 23 12ZM21 12C21 16.9706 16.9706 21 12 21C7.02944 21 3 16.9706 3 12C3 7.02944 7.02944 3 12 3C16.9706 3 21 7.02944 21 12Z"
                    fill="currentColor"
                ></path>
            </svg>
        },
    };

    view! {
        <li class="col-span-1 divide-y divide-gray-200 rounded-lg bg-white shadow cursor-pointer hover:shadow-indigo hover:shadow-[rgba(79,70,229,0.5)_0px_0px_4px_4px]">
            <div class="flex w-full items-center justify-between space-x-6 p-6">
                <div class="flex-1 truncate">
                    <div class="flex items-center space-x-3">
                        <h3
                            class="truncate text-sm font-medium text-gray-900"
                            title=format!("{:?}", device.last_seen)
                        >
                            {&device.name}
                        </h3>
                        <span class="inline-flex flex-shrink-0 items-center rounded-full bg-green-50 px-1.5 py-0.5 text-xs font-medium text-green-700 ring-1 ring-inset ring-green-600/20">
                            {icon}
                        </span>
                    </div>
                    <p class="mt-1 truncate text-sm text-gray-500">{&device.ip}</p>
                </div>
                {children()}
            </div>
        </li>
    }
}
