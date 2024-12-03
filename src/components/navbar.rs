use {
    leptos::{svg::Svg, *},
    leptos_router::A,
};

pub struct NavbarItem {
    pub path: String,
    pub text: String,
    pub image: HtmlElement<Svg>,
}

#[component]
pub fn Navbar() -> impl IntoView {
    let navbar_items = [
        NavbarItem {
            path: "/".to_string(),
            text: "Dashboard".to_string(),
            image: view! {
                <svg
                    class="h-7 w-7 shrink-0"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke-width="1.5"
                    stroke="currentColor"
                    aria-hidden="true"
                >
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        d="M2.25 12l8.954-8.955c.44-.439 1.152-.439 1.591 0L21.75 12M4.5 9.75v10.125c0 .621.504 1.125 1.125 1.125H9.75v-4.875c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125V21h4.125c.621 0 1.125-.504 1.125-1.125V9.75M8.25 21h8.25"
                    ></path>
                </svg>
            },
        },
        NavbarItem {
            path: "/devices".to_owned(),
            text: "Devices".to_owned(),
            image: view! {
                <svg 
                    xmlns="http://www.w3.org/2000/svg" 
                    viewBox="0 0 32 32" 
                    class="h-7 w-7 shrink-0"
                    stroke="currentColor"
                    fill="currentColor"
                    stroke-width=".5"
                >
                    <path d="M11 8C11 7.73478 11.1054 7.48043 11.2929 7.29289C11.4804 7.10536 11.7348 7 12 7H27C27.2652 7 27.5196 7.10536 27.7071 7.29289C27.8946 7.48043 28 7.73478 28 8C28 8.26522 27.8946 8.51957 27.7071 8.70711C27.5196 8.89464 27.2652 9 27 9H12C11.7348 9 11.4804 8.89464 11.2929 8.70711C11.1054 8.51957 11 8.26522 11 8ZM27 15H12C11.7348 15 11.4804 15.1054 11.2929 15.2929C11.1054 15.4804 11 15.7348 11 16C11 16.2652 11.1054 16.5196 11.2929 16.7071C11.4804 16.8946 11.7348 17 12 17H27C27.2652 17 27.5196 16.8946 27.7071 16.7071C27.8946 16.5196 28 16.2652 28 16C28 15.7348 27.8946 15.4804 27.7071 15.2929C27.5196 15.1054 27.2652 15 27 15ZM27 23H12C11.7348 23 11.4804 23.1054 11.2929 23.2929C11.1054 23.4804 11 23.7348 11 24C11 24.2652 11.1054 24.5196 11.2929 24.7071C11.4804 24.8946 11.7348 25 12 25H27C27.2652 25 27.5196 24.8946 27.7071 24.7071C27.8946 24.5196 28 24.2652 28 24C28 23.7348 27.8946 23.4804 27.7071 23.2929C27.5196 23.1054 27.2652 23 27 23ZM7 7H5C4.73478 7 4.48043 7.10536 4.29289 7.29289C4.10536 7.48043 4 7.73478 4 8C4 8.26522 4.10536 8.51957 4.29289 8.70711C4.48043 8.89464 4.73478 9 5 9H7C7.26522 9 7.51957 8.89464 7.70711 8.70711C7.89464 8.51957 8 8.26522 8 8C8 7.73478 7.89464 7.48043 7.70711 7.29289C7.51957 7.10536 7.26522 7 7 7ZM7 15H5C4.73478 15 4.48043 15.1054 4.29289 15.2929C4.10536 15.4804 4 15.7348 4 16C4 16.2652 4.10536 16.5196 4.29289 16.7071C4.48043 16.8946 4.73478 17 5 17H7C7.26522 17 7.51957 16.8946 7.70711 16.7071C7.89464 16.5196 8 16.2652 8 16C8 15.7348 7.89464 15.4804 7.70711 15.2929C7.51957 15.1054 7.26522 15 7 15ZM7 23H5C4.73478 23 4.48043 23.1054 4.29289 23.2929C4.10536 23.4804 4 23.7348 4 24C4 24.2652 4.10536 24.5196 4.29289 24.7071C4.48043 24.8946 4.73478 25 5 25H7C7.26522 25 7.51957 24.8946 7.70711 24.7071C7.89464 24.5196 8 24.2652 8 24C8 23.7348 7.89464 23.4804 7.70711 23.2929C7.51957 23.1054 7.26522 23 7 23Z" />
                </svg>
            },
        },
        NavbarItem {
            path: "/actions".to_owned(),
            text: "Actions".to_owned(),
            image: view! {
                <svg
                    xmlns="http://www.w3.org/2000/svg"
                    class="h-7 w-7 shrink-0"
                    stroke="currentColor"
                    fill="currentColor"
                    stroke-width=".5"
                    viewBox="0 0 31 30"
                >
                    <path
                        d="M24.875 3.75H22.0625V2.8125C22.0625 2.56386 21.9637 2.3254 21.7879 2.14959C21.6121 1.97377 21.3736 1.875 21.125 1.875C20.8764 1.875 20.6379 1.97377 20.4621 2.14959C20.2863 2.3254 20.1875 2.56386 20.1875 2.8125V3.75H10.8125V2.8125C10.8125 2.56386 10.7137 2.3254 10.5379 2.14959C10.3621 1.97377 10.1236 1.875 9.875 1.875C9.62636 1.875 9.3879 1.97377 9.21209 2.14959C9.03627 2.3254 8.9375 2.56386 8.9375 2.8125V3.75H6.125C5.62772 3.75 5.15081 3.94754 4.79917 4.29917C4.44754 4.65081 4.25 5.12772 4.25 5.625V24.375C4.25 24.8723 4.44754 25.3492 4.79917 25.7008C5.15081 26.0525 5.62772 26.25 6.125 26.25H24.875C25.3723 26.25 25.8492 26.0525 26.2008 25.7008C26.5525 25.3492 26.75 24.8723 26.75 24.375V5.625C26.75 5.12772 26.5525 4.65081 26.2008 4.29917C25.8492 3.94754 25.3723 3.75 24.875 3.75ZM8.9375 5.625V6.5625C8.9375 6.81114 9.03627 7.0496 9.21209 7.22541C9.3879 7.40123 9.62636 7.5 9.875 7.5C10.1236 7.5 10.3621 7.40123 10.5379 7.22541C10.7137 7.0496 10.8125 6.81114 10.8125 6.5625V5.625H20.1875V6.5625C20.1875 6.81114 20.2863 7.0496 20.4621 7.22541C20.6379 7.40123 20.8764 7.5 21.125 7.5C21.3736 7.5 21.6121 7.40123 21.7879 7.22541C21.9637 7.0496 22.0625 6.81114 22.0625 6.5625V5.625H24.875V9.375H6.125V5.625H8.9375ZM24.875 24.375H6.125V11.25H24.875V24.375ZM19.25 17.8125C19.25 18.0611 19.1512 18.2996 18.9754 18.4754C18.7996 18.6512 18.5611 18.75 18.3125 18.75H16.4375V20.625C16.4375 20.8736 16.3387 21.1121 16.1629 21.2879C15.9871 21.4637 15.7486 21.5625 15.5 21.5625C15.2514 21.5625 15.0129 21.4637 14.8371 21.2879C14.6613 21.1121 14.5625 20.8736 14.5625 20.625V18.75H12.6875C12.4389 18.75 12.2004 18.6512 12.0246 18.4754C11.8488 18.2996 11.75 18.0611 11.75 17.8125C11.75 17.5639 11.8488 17.3254 12.0246 17.1496C12.2004 16.9738 12.4389 16.875 12.6875 16.875H14.5625V15C14.5625 14.7514 14.6613 14.5129 14.8371 14.3371C15.0129 14.1613 15.2514 14.0625 15.5 14.0625C15.7486 14.0625 15.9871 14.1613 16.1629 14.3371C16.3387 14.5129 16.4375 14.7514 16.4375 15V16.875H18.3125C18.5611 16.875 18.7996 16.9738 18.9754 17.1496C19.1512 17.3254 19.25 17.5639 19.25 17.8125Z"
                    ></path>
                </svg>
            },
        },
        NavbarItem {
            path: "/integrations".to_string(),
            text: "Integrations".to_string(),
            image: view! {
                <svg 
                    xmlns="http://www.w3.org/2000/svg"
                    class="h-7 w-7 shrink-0"
                    viewBox="0 0 31 30" 
                    stroke="currentColor"
                    fill="currentColor"
                    stroke-width=".5"
                >
                    <path d="M23.0586 20.0941C24.2441 19.0983 25.1912 17.8492 25.83 16.4389C26.4689 15.0285 26.7832 13.4929 26.75 11.9449C26.6328 6.58829 22.3426 2.17149 16.9953 1.89024C15.6156 1.81515 14.2349 2.01808 12.9352 2.48697C11.6355 2.95587 10.4432 3.68118 9.42925 4.61983C8.4153 5.55847 7.60031 6.6913 7.03269 7.95105C6.46506 9.2108 6.15639 10.5718 6.12498 11.9531L3.49413 17.0133C3.48358 17.0344 3.47303 17.0555 3.46366 17.0766C3.27504 17.5162 3.2616 18.0113 3.42608 18.4605C3.59056 18.9097 3.92052 19.2791 4.34842 19.493L4.37772 19.5059L7.06248 20.7352V24.375C7.06248 24.8723 7.26003 25.3492 7.61166 25.7008C7.96329 26.0525 8.4402 26.25 8.93748 26.25H14.5625C14.8111 26.25 15.0496 26.1512 15.2254 25.9754C15.4012 25.7996 15.5 25.5611 15.5 25.3125C15.5 25.0639 15.4012 24.8254 15.2254 24.6496C15.0496 24.4738 14.8111 24.375 14.5625 24.375H8.93748V20.134C8.93762 19.9543 8.88611 19.7783 8.78908 19.6271C8.69206 19.4758 8.55361 19.3557 8.39022 19.2809L5.18748 17.8125L7.891 12.6164C7.96147 12.4844 7.99886 12.3372 7.99998 12.1875C7.99971 10.2754 8.64891 8.41991 9.84121 6.92507C11.0335 5.43022 12.6982 4.38469 14.5625 3.95977V5.78673C13.937 6.00788 13.4098 6.44304 13.0741 7.0153C12.7384 7.58756 12.6158 8.26007 12.728 8.91398C12.8402 9.56788 13.1799 10.1611 13.6872 10.5887C14.1944 11.0163 14.8365 11.2509 15.5 11.2509C16.1634 11.2509 16.8055 11.0163 17.3128 10.5887C17.82 10.1611 18.1598 9.56788 18.272 8.91398C18.3842 8.26007 18.2616 7.58756 17.9259 7.0153C17.5902 6.44304 17.063 6.00788 16.4375 5.78673V3.75001C16.5898 3.75001 16.7422 3.75001 16.8945 3.76173C18.557 3.85696 20.1546 4.44026 21.4873 5.43867C22.82 6.43707 23.8288 7.80627 24.3875 9.37501H22.0625C21.925 9.37496 21.7891 9.40516 21.6645 9.46349C21.54 9.52182 21.4298 9.60683 21.3418 9.71251L18.3523 13.3008C17.7289 13.0685 17.0433 13.0641 16.4169 13.2885C15.7906 13.5129 15.2637 13.9516 14.9296 14.527C14.5955 15.1023 14.4756 15.7773 14.5912 16.4326C14.7067 17.0878 15.0503 17.6811 15.5611 18.1074C16.0719 18.5338 16.717 18.7658 17.3824 18.7624C18.0477 18.7589 18.6904 18.5203 19.1968 18.0887C19.7032 17.6571 20.0406 17.0604 20.1494 16.404C20.2582 15.7476 20.1314 15.0739 19.7914 14.502L22.5019 11.25H24.8211C24.8492 11.4938 24.8664 11.7399 24.8726 11.9883C24.9019 13.3311 24.6113 14.6615 24.0248 15.8699C23.4384 17.0782 22.573 18.1297 21.5 18.9375C21.3678 19.0366 21.2641 19.1689 21.1995 19.321C21.1349 19.4731 21.1116 19.6396 21.132 19.8035L22.0695 27.3035C22.098 27.5299 22.2082 27.7382 22.3792 27.8892C22.5503 28.0402 22.7706 28.1236 22.9988 28.1238C23.038 28.1238 23.0771 28.1215 23.116 28.1168C23.2382 28.1016 23.3562 28.0625 23.4633 28.0016C23.5703 27.9408 23.6644 27.8595 23.74 27.7623C23.8156 27.6651 23.8714 27.554 23.9041 27.4352C23.9367 27.3165 23.9457 27.1925 23.9305 27.0703L23.0586 20.0941ZM15.5 9.37501C15.3146 9.37501 15.1333 9.32002 14.9791 9.21701C14.825 9.114 14.7048 8.96758 14.6338 8.79627C14.5629 8.62497 14.5443 8.43647 14.5805 8.25461C14.6167 8.07275 14.706 7.90571 14.8371 7.7746C14.9682 7.64348 15.1352 7.5542 15.3171 7.51802C15.4989 7.48185 15.6874 7.50041 15.8587 7.57137C16.0301 7.64233 16.1765 7.76249 16.2795 7.91666C16.3825 8.07083 16.4375 8.25209 16.4375 8.43751C16.4375 8.68615 16.3387 8.9246 16.1629 9.10042C15.9871 9.27624 15.7486 9.37501 15.5 9.37501ZM17.375 16.875C17.1896 16.875 17.0083 16.82 16.8541 16.717C16.7 16.614 16.5798 16.4676 16.5088 16.2963C16.4379 16.125 16.4193 15.9365 16.4555 15.7546C16.4917 15.5728 16.581 15.4057 16.7121 15.2746C16.8432 15.1435 17.0102 15.0542 17.1921 15.018C17.3739 14.9818 17.5624 15.0004 17.7337 15.0714C17.9051 15.1423 18.0515 15.2625 18.1545 15.4167C18.2575 15.5708 18.3125 15.7521 18.3125 15.9375C18.3125 16.1861 18.2137 16.4246 18.0379 16.6004C17.8621 16.7762 17.6236 16.875 17.375 16.875Z" />
                </svg>
            },
        },
        NavbarItem {
            path: "/settings".to_string(),
            text: "Settings".to_string(),
            image: view! {
                <svg
                    class="h-7 w-7 shrink-0"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke-width="1.5"
                    stroke="currentColor"
                    aria-hidden="true"
                >
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.324.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 011.37.49l1.296 2.247a1.125 1.125 0 01-.26 1.431l-1.003.827c-.293.24-.438.613-.431.992a6.759 6.759 0 010 .255c-.007.378.138.75.43.99l1.005.828c.424.35.534.954.26 1.43l-1.298 2.247a1.125 1.125 0 01-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.57 6.57 0 01-.22.128c-.331.183-.581.495-.644.869l-.213 1.28c-.09.543-.56.941-1.11.941h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 01-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 01-1.369-.49l-1.297-2.247a1.125 1.125 0 01.26-1.431l1.004-.827c.292-.24.437-.613.43-.992a6.932 6.932 0 010-.255c.007-.378-.138-.75-.43-.99l-1.004-.828a1.125 1.125 0 01-.26-1.43l1.297-2.247a1.125 1.125 0 011.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.281z"
                    ></path>
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                    ></path>
                </svg>
            },
        },
    ].into_iter()
    .map(|navbar_item| {
        view! {
            <li>
                <A
                    href=navbar_item.path
                    class="nav-link text-gray-400 group flex gap-x-3 rounded-md p-3 text-sm leading-6 font-semibold"
                >
                    {navbar_item.image}
                    <span class="sr-only">{navbar_item.text}</span>
                </A>
            </li>
        }
    }).collect::<Vec<_>>();

    view! {
        <div class="hidden lg:fixed lg:inset-y-0 lg:left-0 lg:z-50 lg:block lg:w-20 lg:overflow-y-auto lg:bg-gray-900 lg:pb-4">
            <div class="flex h-16 shrink-0 items-center justify-center">
                <img class="h-8 w-auto rounded-full" src="/icon.png" alt="Iron Nest"/>
            </div>
            <nav class="mt-8">
                <ul role="list" class="flex flex-col items-center space-y-1">
                    {navbar_items}
                </ul>
            </nav>
        </div>
    }
}
