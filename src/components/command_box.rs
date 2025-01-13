use leptos::prelude::*;
use crate::server::openai::HandleAssistantCommand;

#[component]
pub fn CommandBox() -> impl IntoView {
    let handle_assistant = ServerAction::<HandleAssistantCommand>::new();
    let value = handle_assistant.value();

    view! {
        <div class="col-span-3 h-[264px] h-full flex flex-col panel-form bg-white p-2 rounded-md shadow-lg">
            <ActionForm action=handle_assistant>
                <div class="flex flex-col h-full">
                    <textarea
                        name="text"
                        class="resize-none rounded-md border-2 p-2 bg-white border-blue-500 flex-grow h-full w-full"
                        placeholder="Enter text and hit enter"
                    ></textarea>
                    <div class="flex-shrink-0 mt-2">
                        <button
                            type="submit"
                            class="inline-flex items-center rounded-md bg-indigo-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
                        >
                            Submit command
                        </button>
                    </div>
                    <div class="mt-2">{value}</div>
                </div>
            </ActionForm>
        </div>
    }
}
