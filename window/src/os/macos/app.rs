use crate::connection::ConnectionOps;
use crate::macos::menu::RepresentedItem;
use crate::macos::{nsstring, nsstring_to_str};
use crate::menu::{Menu, MenuItem};
use crate::{ApplicationEvent, Connection};
use cocoa::appkit::NSApplicationTerminateReply;
use cocoa::appkit::{NSFilenamesPboardType, NSApp, NSPasteboard};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSArray, NSInteger, NSFastEnumeration};
use config::keyassignment::KeyAssignment;
use config::WindowCloseConfirmation;
use objc::declare::ClassDecl;
use objc::rc::StrongPtr;
use objc::runtime::{Class, Object, Sel, BOOL, NO, YES};
use objc::*;
use std::process::Command;

const CLS_NAME: &str = "WezTermAppDelegate";

extern "C" fn application_should_terminate(
    _self: &mut Object,
    _sel: Sel,
    _app: *mut Object,
) -> u64 {
    log::debug!("application termination requested");
    unsafe {
        match config::configuration().window_close_confirmation {
            WindowCloseConfirmation::NeverPrompt => terminate_now(),
            WindowCloseConfirmation::AlwaysPrompt => {
                let alert: id = msg_send![class!(NSAlert), alloc];
                let alert: id = msg_send![alert, init];
                let message_text = nsstring("Terminate WezTerm?");
                let info_text = nsstring("Detach and close all panes and terminate wezterm?");
                let cancel = nsstring("Cancel");
                let ok = nsstring("Ok");

                let () = msg_send![alert, setMessageText: message_text];
                let () = msg_send![alert, setInformativeText: info_text];
                let () = msg_send![alert, addButtonWithTitle: cancel];
                let () = msg_send![alert, addButtonWithTitle: ok];
                #[allow(non_upper_case_globals)]
                const NSModalResponseCancel: NSInteger = 1000;
                #[allow(non_upper_case_globals, dead_code)]
                const NSModalResponseOK: NSInteger = 1001;
                let result: NSInteger = msg_send![alert, runModal];
                log::info!("alert result is {result}");

                if result == NSModalResponseCancel {
                    NSApplicationTerminateReply::NSTerminateCancel as u64
                } else {
                    terminate_now()
                }
            }
        }
    }
}

fn terminate_now() -> u64 {
    if let Some(conn) = Connection::get() {
        conn.terminate_message_loop();
    }
    NSApplicationTerminateReply::NSTerminateNow as u64
}

extern "C" fn application_will_finish_launching(
    _self: &mut Object,
    _sel: Sel,
    _notif: *mut Object,
) {
    log::debug!("application_will_finish_launching");
}

extern "C" fn application_did_finish_launching(this: &mut Object, _sel: Sel, _notif: *mut Object) {
    log::debug!("application_did_finish_launching");
    unsafe {
        (*this).set_ivar("launched", YES);

        let ns_app = NSApp();
        let services_menu = Menu::new_with_title("Services");
        services_menu.assign_as_services_menu();
        let () = msg_send![ns_app, setServicesProvider: this];
        let send_types = NSArray::arrayWithObject(nil, NSFilenamesPboardType);
        let () = msg_send![ns_app, registerServicesMenuSendTypes: send_types returnTypes: nil];
    }
    register_finder_integration();
}

extern "C" fn application_open_untitled_file(
    this: &mut Object,
    _sel: Sel,
    _app: *mut Object,
) -> BOOL {
    let launched: BOOL = unsafe { *this.get_ivar("launched") };
    log::debug!("application_open_untitled_file launched={launched}");
    if let Some(conn) = Connection::get() {
        if launched == YES {
            conn.dispatch_app_event(ApplicationEvent::PerformKeyAssignment(
                KeyAssignment::SpawnWindow,
            ));
        }
        return YES;
    }
    NO
}

extern "C" fn wezterm_perform_key_assignment(
    _self: &mut Object,
    _sel: Sel,
    menu_item: *mut Object,
) {
    let menu_item = crate::os::macos::menu::MenuItem::with_menu_item(menu_item);
    // Safe because weztermPerformKeyAssignment: is only used with KeyAssignment
    let action = menu_item.get_represented_item();
    log::debug!("wezterm_perform_key_assignment {action:?}",);
    match action {
        Some(RepresentedItem::KeyAssignment(action)) => {
            if let Some(conn) = Connection::get() {
                conn.dispatch_app_event(ApplicationEvent::PerformKeyAssignment(action));
            }
        }
        None => {}
    }
}

extern "C" fn application_open_file(
    this: &mut Object,
    _sel: Sel,
    _app: *mut Object,
    file_name: *mut Object,
) {
    let launched: BOOL = unsafe { *this.get_ivar("launched") };
    if launched == YES {
        let file_name = unsafe { nsstring_to_str(file_name) }.to_string();
        let path = std::path::PathBuf::from(&file_name);
        if let Some(conn) = Connection::get() {
            log::debug!("application_open_file {file_name}");
            if path.is_dir() {
                conn.dispatch_app_event(ApplicationEvent::OpenDirectory(path));
            } else if is_command_script(&path) {
                conn.dispatch_app_event(ApplicationEvent::OpenCommandScript(file_name));
            } else if let Some(parent) = path.parent() {
                conn.dispatch_app_event(ApplicationEvent::OpenDirectory(parent.to_path_buf()));
            } else {
                conn.dispatch_app_event(ApplicationEvent::OpenCommandScript(file_name));
            }
        }
    }
}

extern "C" fn application_open_in_unterm(
    this: &mut Object,
    _sel: Sel,
    pboard: *mut Object,
    _user_data: *mut Object,
    _error: *mut Object,
) {
    let launched: BOOL = unsafe { *this.get_ivar("launched") };
    if launched != YES {
        return;
    }

    let paths = unsafe {
        let filenames = NSPasteboard::propertyListForType(pboard, NSFilenamesPboardType);
        if filenames.is_null() {
            Vec::new()
        } else {
            filenames
                .iter()
                .map(|file| {
                    let path = nsstring_to_str(file);
                    std::path::PathBuf::from(path)
                })
                .collect::<Vec<_>>()
        }
    };

    let path: std::path::PathBuf = match paths.into_iter().next() {
        Some(path) => {
            if path.is_dir() {
                path
            } else {
                path.parent()
                    .map(|p: &std::path::Path| p.to_path_buf())
                    .unwrap_or(path)
            }
        }
        None => return,
    };

    if let Some(conn) = Connection::get() {
        log::debug!("application_open_in_unterm cwd={}", path.display());
        conn.dispatch_app_event(ApplicationEvent::OpenDirectory(path));
    }
}

fn is_command_script(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("command" | "sh" | "zsh" | "bash" | "fish" | "tool")
    )
}

fn register_finder_integration() {
    let Some(app_path) = main_bundle_path() else {
        return;
    };

    std::thread::spawn(move || {
        let lsregister = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";
        let _ = Command::new(lsregister).arg("-f").arg(&app_path).status();

        let appex = std::path::Path::new(&app_path)
            .join("Contents")
            .join("PlugIns")
            .join("UntermFinderSync.appex");
        if appex.exists() {
            let _ = Command::new("pluginkit").arg("-a").arg(&appex).status();
            let _ = Command::new("pluginkit")
                .args(["-e", "use", "-i", "ai.unzoo.unterm.finder-sync"])
                .status();
        }
    });
}

fn main_bundle_path() -> Option<String> {
    unsafe {
        let bundle: id = msg_send![class!(NSBundle), mainBundle];
        if bundle.is_null() {
            return None;
        }
        let path: id = msg_send![bundle, bundlePath];
        if path.is_null() {
            return None;
        }
        Some(nsstring_to_str(path).to_string())
    }
}

extern "C" fn application_dock_menu(
    _self: &mut Object,
    _sel: Sel,
    _app: *mut Object,
) -> *mut Object {
    let dock_menu = Menu::new_with_title("");
    let new_window_item =
        MenuItem::new_with("New Window", Some(sel!(weztermPerformKeyAssignment:)), "");
    new_window_item
        .set_represented_item(RepresentedItem::KeyAssignment(KeyAssignment::SpawnWindow));
    dock_menu.add_item(&new_window_item);
    dock_menu.autorelease()
}

fn get_class() -> &'static Class {
    Class::get(CLS_NAME).unwrap_or_else(|| {
        let mut cls = ClassDecl::new(CLS_NAME, class!(NSObject))
            .expect("Unable to register application delegate class");

        cls.add_ivar::<BOOL>("launched");

        unsafe {
            cls.add_method(
                sel!(applicationShouldTerminate:),
                application_should_terminate as extern "C" fn(&mut Object, Sel, *mut Object) -> u64,
            );
            cls.add_method(
                sel!(applicationWillFinishLaunching:),
                application_will_finish_launching as extern "C" fn(&mut Object, Sel, *mut Object),
            );
            cls.add_method(
                sel!(applicationDidFinishLaunching:),
                application_did_finish_launching as extern "C" fn(&mut Object, Sel, *mut Object),
            );
            cls.add_method(
                sel!(application:openFile:),
                application_open_file as extern "C" fn(&mut Object, Sel, *mut Object, *mut Object),
            );
            cls.add_method(
                sel!(openInUnterm:userData:error:),
                application_open_in_unterm
                    as extern "C" fn(&mut Object, Sel, *mut Object, *mut Object, *mut Object),
            );
            cls.add_method(
                sel!(applicationDockMenu:),
                application_dock_menu
                    as extern "C" fn(&mut Object, Sel, *mut Object) -> *mut Object,
            );
            cls.add_method(
                sel!(weztermPerformKeyAssignment:),
                wezterm_perform_key_assignment as extern "C" fn(&mut Object, Sel, *mut Object),
            );
            cls.add_method(
                sel!(applicationOpenUntitledFile:),
                application_open_untitled_file
                    as extern "C" fn(&mut Object, Sel, *mut Object) -> BOOL,
            );
        }

        cls.register()
    })
}

pub fn create_app_delegate() -> StrongPtr {
    let cls = get_class();
    unsafe {
        let delegate: *mut Object = msg_send![cls, alloc];
        let delegate: *mut Object = msg_send![delegate, init];
        (*delegate).set_ivar("launched", NO);
        StrongPtr::new(delegate)
    }
}
