//! # Examples
//!
//! ```rust,no_run
//! use ashpd::desktop::remote_desktop::{
//!     CreateRemoteOptions, DeviceType, KeyState, RemoteDesktopProxy,
//!     SelectDevicesOptions, SelectedDevices, StartRemoteOptions,
//! };
//! use ashpd::{BasicResponse, HandleToken, WindowIdentifier};
//! use std::collections::HashMap;
//! use std::convert::TryFrom;
//!
//! async fn run() -> Result<(), ashpd::Error> {
//!     let connection = zbus::azync::Connection::new_session().await?;
//!     let proxy = RemoteDesktopProxy::new(&connection).await?;
//!
//!     let session = proxy.create_session(
//!         CreateRemoteOptions::default()
//!             .session_handle_token(HandleToken::try_from("token").unwrap()),
//!     ).await?;
//!
//!     let request = proxy.select_devices(&session,
//!         SelectDevicesOptions::default().types(DeviceType::Keyboard | DeviceType::Pointer),
//!     ).await?;
//!     let _ = request.receive_response::<BasicResponse>().await?;
//!
//!     let request = proxy.start(
//!         &session,
//!         WindowIdentifier::default(),
//!         StartRemoteOptions::default(),
//!     ).await?;
//!     let devices = request.receive_response::<SelectedDevices>().await?;
//!     println!("{:#?}", devices);
//!
//!     // 13 for Enter key code
//!     proxy.notify_keyboard_keycode(&session, HashMap::new(), 13, KeyState::Pressed).await?;
//!
//!     Ok(())
//! }
//! ```
use std::collections::HashMap;

use enumflags2::BitFlags;
use serde_repr::{Deserialize_repr, Serialize_repr};
use zvariant::{OwnedObjectPath, Value};
use zvariant_derive::{DeserializeDict, SerializeDict, Type, TypeDict};

use crate::{Error, HandleToken, RequestProxy, SessionProxy, WindowIdentifier};

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Type)]
#[repr(u32)]
/// The keyboard key state.
pub enum KeyState {
    /// The key is pressed.
    Pressed = 0,
    /// The key is released..
    Released = 1,
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, BitFlags, Clone, Copy, Type)]
#[repr(u32)]
/// A bit flag for the available devices.
pub enum DeviceType {
    /// A keyboard.
    Keyboard = 1,
    /// A mouse pointer.
    Pointer = 2,
    /// A touchscreen
    Touchscreen = 4,
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Type)]
#[repr(u32)]
/// The available axis.
pub enum Axis {
    /// Vertical axis.
    Vertical = 0,
    /// Horizontal axis.
    Horizontal = 1,
}

#[derive(SerializeDict, DeserializeDict, TypeDict, Debug, Default)]
/// Specified options on a create a remote session request.
pub struct CreateRemoteOptions {
    /// A string that will be used as the last element of the handle.
    handle_token: Option<HandleToken>,
    /// A string that will be used as the last element of the session handle.
    session_handle_token: Option<HandleToken>,
}

impl CreateRemoteOptions {
    /// Sets the handle token.
    pub fn handle_token(mut self, handle_token: HandleToken) -> Self {
        self.handle_token = Some(handle_token);
        self
    }

    /// Sets the session handle token.
    pub fn session_handle_token(mut self, session_handle_token: HandleToken) -> Self {
        self.session_handle_token = Some(session_handle_token);
        self
    }
}

#[derive(SerializeDict, DeserializeDict, TypeDict, Debug)]
/// A response to a `create_session` request.
struct CreateSession {
    /// A string that will be used as the last element of the session handle.
    pub(crate) session_handle: OwnedObjectPath,
}

#[derive(SerializeDict, DeserializeDict, TypeDict, Debug, Default)]
/// Specified options on a select devices request.
pub struct SelectDevicesOptions {
    /// A string that will be used as the last element of the handle.
    handle_token: Option<HandleToken>,
    /// The device types to request remote controlling of. Default is all.
    types: Option<BitFlags<DeviceType>>,
}

impl SelectDevicesOptions {
    /// Sets the handle token.
    pub fn handle_token(mut self, handle_token: HandleToken) -> Self {
        self.handle_token = Some(handle_token);
        self
    }

    /// Sets the device types to request remote controlling of.
    pub fn types(mut self, types: BitFlags<DeviceType>) -> Self {
        self.types = Some(types);
        self
    }
}

#[derive(SerializeDict, DeserializeDict, TypeDict, Debug, Default)]
/// Specified options on a start remote desktop request.
pub struct StartRemoteOptions {
    /// A string that will be used as the last element of the handle.
    handle_token: Option<HandleToken>,
}

impl StartRemoteOptions {
    /// Sets the handle token.
    pub fn handle_token(mut self, handle_token: HandleToken) -> Self {
        self.handle_token = Some(handle_token);
        self
    }
}

#[derive(SerializeDict, DeserializeDict, TypeDict, Debug, Default)]
/// A response to a select device request.
pub struct SelectedDevices {
    /// The selected devices.
    pub devices: BitFlags<DeviceType>,
}

/// The interface lets sandboxed applications create remote desktop sessions.
pub struct RemoteDesktopProxy<'a>(zbus::azync::Proxy<'a>);

impl<'a> RemoteDesktopProxy<'a> {
    pub async fn new(
        connection: &zbus::azync::Connection,
    ) -> Result<RemoteDesktopProxy<'a>, Error> {
        let proxy = zbus::ProxyBuilder::new_bare(connection)
            .interface("org.freedesktop.portal.RemoteDesktop")
            .path("/org/freedesktop/portal/desktop")?
            .destination("org.freedesktop.portal.Desktop")
            .build_async()
            .await?;
        Ok(Self(proxy))
    }

    /// Create a remote desktop session.
    /// A remote desktop session is used to allow remote controlling a desktop
    /// session. It can also be used together with a screen cast session.
    ///
    /// # Arguments
    ///
    /// * `options` - A [`CreateRemoteOptions`].
    ///
    /// [`CreateRemoteOptions`]: ./struct.CreateRemoteOptions.html
    pub async fn create_session(
        &self,
        options: CreateRemoteOptions,
    ) -> Result<SessionProxy<'a>, Error> {
        let path: zvariant::OwnedObjectPath = self
            .0
            .call_method("CreateSession", &(options))
            .await?
            .body()?;
        let request = RequestProxy::new(self.0.connection(), path).await?;
        let session = request.receive_response::<CreateSession>().await?;
        SessionProxy::new(self.0.connection(), session.session_handle).await
    }

    /// Select input devices to remote control.
    ///
    /// # Arguments
    ///
    /// * `session` - A [`SessionProxy`].
    /// * `options` - [`SelectDevicesOptions`].
    ///
    /// [`SelectDevicesOptions`]: ../struct.SelectDevicesOptions.html
    /// [`SessionProxy`]: ../../session/struct.SessionProxy.html
    pub async fn select_devices(
        &self,
        session: &SessionProxy<'_>,
        options: SelectDevicesOptions,
    ) -> Result<RequestProxy<'a>, Error> {
        let path: zvariant::OwnedObjectPath = self
            .0
            .call_method("SelectDevices", &(session, options))
            .await?
            .body()?;
        RequestProxy::new(self.0.connection(), path).await
    }

    ///  Start the remote desktop session.
    ///
    /// This will typically result in the portal presenting a dialog letting
    /// the user select what to share, including devices and optionally screen
    /// content if screen cast sources was selected.
    ///
    /// # Arguments
    ///
    /// * `session` - A [`SessionProxy`].
    /// * `parent_window` - The application window identifier.
    /// * `options` - [`StartRemoteOptions`].
    ///
    /// [`StartRemoteOptions`]: ../struct.StartRemoteOptions.html
    /// [`SessionProxy`]: ../../session/struct.SessionProxy.html
    pub async fn start(
        &self,
        session: &SessionProxy<'_>,
        parent_window: WindowIdentifier,
        options: StartRemoteOptions,
    ) -> Result<RequestProxy<'a>, Error> {
        let path: zvariant::OwnedObjectPath = self
            .0
            .call_method("Start", &(session, parent_window, options))
            .await?
            .body()?;
        RequestProxy::new(self.0.connection(), path).await
    }

    /// Notify keyboard code.
    /// May only be called if KEYBOARD access was provided after starting the
    /// session.
    ///
    /// # Arguments
    ///
    /// * `session` - A [`SessionProxy`].
    /// * `options` - ?
    /// * `keycode` - Keyboard code that was pressed or released.
    /// * `state` - The new state of the keyboard code.
    ///
    /// [`SessionProxy`]: ../../session/struct.SessionProxy.html
    ///
    /// FIXME: figure out the options we can take here
    pub async fn notify_keyboard_keycode(
        &self,
        session: &SessionProxy<'_>,
        options: HashMap<&str, Value<'_>>,
        keycode: i32,
        state: KeyState,
    ) -> Result<(), Error> {
        self.0
            .call_method("NotifyKeyboardKeycode", &(session, options, keycode, state))
            .await?
            .body()
            .map_err(From::from)
    }

    /// Notify keyboard symbol.
    /// May only be called if KEYBOARD access was provided after starting the
    /// session.
    ///
    /// # Arguments
    ///
    /// * `session` - A [`SessionProxy`].
    /// * `options` - ?
    /// * `keysym` - Keyboard symbol that was pressed or released.
    /// * `state` - The new state of the keyboard code.
    ///
    /// [`SessionProxy`]: ../../session/struct.SessionProxy.html
    ///
    /// FIXME: figure out the options we can take here
    pub async fn notify_keyboard_keysym(
        &self,
        session: &SessionProxy<'_>,
        options: HashMap<&str, Value<'_>>,
        keysym: i32,
        state: KeyState,
    ) -> Result<(), Error> {
        self.0
            .call_method("NotifyKeyboardKeysym", &(session, options, keysym, state))
            .await?
            .body()
            .map_err(From::from)
    }

    /// Notify about a new touch up event.
    ///
    /// May only be called if TOUCHSCREEN access was provided after starting the
    /// session.
    ///
    /// # Arguments
    ///
    /// * `session` - A [`SessionProxy`].
    /// * `options` - ?
    /// * `slot` - Touch slot where touch point appeared.
    ///
    /// [`SessionProxy`]: ../../session/struct.SessionProxy.html
    ///
    /// FIXME: figure out the options we can take here
    pub async fn notify_touch_up(
        &self,
        session: &SessionProxy<'_>,
        options: HashMap<&str, Value<'_>>,
        slot: u32,
    ) -> Result<(), Error> {
        self.0
            .call_method("NotifyTouchUp", &(session, options, slot))
            .await?
            .body()
            .map_err(From::from)
    }

    /// Notify about a new touch down event.
    /// The (x, y) position represents the new touch point position in the
    /// streams logical coordinate space.
    ///
    /// May only be called if TOUCHSCREEN access was provided after starting the
    /// session.
    ///
    /// # Arguments
    ///
    /// * `session` - A [`SessionProxy`].
    /// * `options` - ?
    /// * `stream` - The PipeWire stream node the coordinate is relative to.
    /// * `slot` - Touch slot where touch point appeared.
    /// * `x` - Touch down x coordinate.
    /// * `y` - Touch down y coordinate.
    ///
    /// [`SessionProxy`]: ../../session/struct.SessionProxy.html
    ///
    /// FIXME: figure out the options we can take here
    pub async fn notify_touch_down(
        &self,
        session: &SessionProxy<'_>,
        options: HashMap<&str, Value<'_>>,
        stream: u32,
        slot: u32,
        x: f64,
        y: f64,
    ) -> Result<(), Error> {
        self.0
            .call_method("NotifyTouchDown", &(session, options, stream, slot, x, y))
            .await?
            .body()
            .map_err(From::from)
    }

    /// Notify about a new touch motion event.
    /// The (x, y) position represents where the touch point position in the
    /// streams logical coordinate space moved.
    ///
    /// May only be called if TOUCHSCREEN access was provided after starting the
    /// session.
    ///
    /// # Arguments
    ///
    /// * `session` - A [`SessionProxy`].
    /// * `options` - ?
    /// * `stream` - The PipeWire stream node the coordinate is relative to.
    /// * `slot` - Touch slot where touch point appeared.
    /// * `x` - Touch motion x coordinate.
    /// * `y` - Touch motion y coordinate.
    ///
    /// [`SessionProxy`]: ../../session/struct.SessionProxy.html
    ///
    /// FIXME: figure out the options we can take here
    pub async fn notify_touch_motion(
        &self,
        session: &SessionProxy<'_>,
        options: HashMap<&str, Value<'_>>,
        stream: u32,
        slot: u32,
        x: f64,
        y: f64,
    ) -> Result<(), Error> {
        self.0
            .call_method("NotifyTouchMotion", &(session, options, stream, slot, x, y))
            .await?
            .body()
            .map_err(From::from)
    }

    /// Notify about a new absolute pointer motion event.
    /// The (x, y) position represents the new pointer position in the streams
    /// logical coordinate space.
    ///
    /// # Arguments
    ///
    /// * `session` - A [`SessionProxy`].
    /// * `options` - ?
    /// * `stream` - The PipeWire stream node the coordinate is relative to.
    /// * `x` - Pointer motion x coordinate.
    /// * `y` - Pointer motion y coordinate.
    ///
    /// [`SessionProxy`]: ../../session/struct.SessionProxy.html
    ///
    /// FIXME: figure out the options we can take here
    pub async fn notify_pointer_motion_absolute(
        &self,
        session: &SessionProxy<'_>,
        options: HashMap<&str, Value<'_>>,
        stream: u32,
        x: f64,
        y: f64,
    ) -> Result<(), Error> {
        self.0
            .call_method(
                "NotifyPointerMotionAbsolute",
                &(session, options, stream, x, y),
            )
            .await?
            .body()
            .map_err(From::from)
    }

    /// Notify about a new relative pointer motion event.
    /// The (dx, dy) vector represents the new pointer position in the streams
    /// logical coordinate space.
    ///
    /// # Arguments
    ///
    /// * `session` - A [`SessionProxy`].
    /// * `options` - ?
    /// * `dx` - Relative movement on the x axis.
    /// * `dy` - Relative movement on the y axis.
    ///
    /// [`SessionProxy`]: ../../session/struct.SessionProxy.html
    ///
    /// FIXME: figure out the options we can take here
    pub async fn notify_pointer_motion(
        &self,
        session: &SessionProxy<'_>,
        options: HashMap<&str, Value<'_>>,
        dx: f64,
        dy: f64,
    ) -> Result<(), Error> {
        self.0
            .call_method("NotifyPointerMotionAbsolute", &(session, options, dx, dy))
            .await?
            .body()
            .map_err(From::from)
    }

    /// Notify pointer button.
    /// The pointer button is encoded according to Linux Evdev button codes.
    ///
    ///  May only be called if POINTER access was provided after starting the
    /// session.
    ///
    /// # Arguments
    ///
    /// * `session` - A [`SessionProxy`].
    /// * `options` - ?
    /// * `button` - The pointer button was pressed or released.
    /// * `state` - The new state of the keyboard code.
    ///
    /// [`SessionProxy`]: ../../session/struct.SessionProxy.html
    ///
    /// FIXME: figure out the options we can take here
    pub async fn notify_pointer_button(
        &self,
        session: &SessionProxy<'_>,
        options: HashMap<&str, Value<'_>>,
        button: i32,
        state: KeyState,
    ) -> Result<(), Error> {
        self.0
            .call_method("NotifyPointerButton", &(session, options, button, state))
            .await?
            .body()
            .map_err(From::from)
    }

    /// Notify pointer axis discrete.
    /// May only be called if POINTER access was provided after starting the
    /// session.
    ///
    /// # Arguments
    ///
    /// * `session` - A [`SessionProxy`].
    /// * `options` - ?
    /// * `axis` - The axis that was scrolled.
    ///
    /// [`SessionProxy`]: ../../session/struct.SessionProxy.html
    ///
    /// FIXME: figure out the options we can take here
    pub async fn notify_pointer_axis_discrete(
        &self,
        session: &SessionProxy<'_>,
        options: HashMap<&str, Value<'_>>,
        axis: Axis,
        steps: i32,
    ) -> Result<(), Error> {
        self.0
            .call_method(
                "NotifyPointerAxisDiscrete",
                &(session, options, axis, steps),
            )
            .await?
            .body()
            .map_err(From::from)
    }

    /// Notify pointer axis.
    /// The axis movement from a "smooth scroll" device, such as a touchpad.
    /// When applicable, the size of the motion delta should be equivalent to
    /// the motion vector of a pointer motion done using the same advice.
    ///
    /// May only be called if POINTER access was provided after starting the
    /// session.
    ///
    /// # Arguments
    ///
    /// * `session` - A [`SessionProxy`].
    /// * `options` - ?
    /// * `dx` - Relative axis movement on the x axis.
    /// * `dy` - Relative axis movement on the y axis.
    ///
    /// [`SessionProxy`]: ../../session/struct.SessionProxy.html
    ///
    /// FIXME: figure out the options we can take here
    pub async fn notify_pointer_axis(
        &self,
        session: &SessionProxy<'_>,
        options: HashMap<&str, Value<'_>>,
        dx: f64,
        dy: f64,
    ) -> Result<(), Error> {
        self.0
            .call_method("NotifyPointerAxis", &(session, options, dx, dy))
            .await?
            .body()
            .map_err(From::from)
    }

    /// Available source types.
    pub async fn available_device_types(&self) -> Result<BitFlags<DeviceType>, Error> {
        self.0
            .get_property::<BitFlags<DeviceType>>("AvailableDeviceTypes")
            .await
            .map_err(From::from)
    }

    /// The version of this DBus interface.
    pub async fn version(&self) -> Result<u32, Error> {
        self.0
            .get_property::<u32>("version")
            .await
            .map_err(From::from)
    }
}
