use dot_api::{Text, Translator};
use objc2::rc::Retained;
use objc2::runtime::Sel;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
use objc2_foundation::NSString;

pub(crate) fn translate_menu(translator: &impl Translator) {
    let Some(main_thread) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(main_thread);
    let Some(main_menu) = app.mainMenu() else {
        return;
    };
    let services_menu = app.servicesMenu();

    for submenu in main_menu
        .itemArray()
        .iter()
        .filter_map(|item| item.submenu())
    {
        for item in &submenu.itemArray() {
            let opens_services = opens_menu(&item, services_menu.as_deref());
            if let Some(text) = menu_text(item.action(), opens_services) {
                item.setTitle(&NSString::from_str(&translator.translate(text)));
            }
        }
    }
}

fn opens_menu(item: &NSMenuItem, menu: Option<&NSMenu>) -> bool {
    item.submenu()
        .zip(menu)
        .is_some_and(|(submenu, menu)| std::ptr::eq(Retained::as_ptr(&submenu), menu))
}

fn menu_text(action: Option<Sel>, opens_services: bool) -> Option<Text> {
    if opens_services {
        return Some(Text::MenuServices);
    }

    let action = action?;
    [
        (sel!(orderFrontStandardAboutPanel:), Text::MenuAbout),
        (sel!(hide:), Text::MenuHide),
        (sel!(hideOtherApplications:), Text::MenuHideOthers),
        (sel!(unhideAllApplications:), Text::MenuShowAll),
        (sel!(terminate:), Text::MenuQuit),
    ]
    .into_iter()
    .find_map(|(selector, text)| (selector == action).then_some(text))
}

#[cfg(test)]
#[path = "../../../../../tests/dot-desktop/shell/menu.rs"]
mod tests;
