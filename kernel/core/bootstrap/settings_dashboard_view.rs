//! Shared production Network and Nodes Settings painters; inputs are immutable display snapshots, not services.
use super::DisplayDevice;
use super::primitives::UI_FONT_CELL_HEIGHT;
#[derive(Clone,Copy)] pub(super) enum VerificationState {Pending,Local,Remote,Confirmed}
#[derive(Clone,Copy)] pub(super) struct NodeVerificationSnapshot {pub code:u32,pub fingerprint:[u8;32],pub remaining:u64,pub state:VerificationState}
#[derive(Clone,Copy)] pub(super) struct NodeDashboardSnapshot {pub identity_ready:bool,pub counts:[usize;8],pub selected:bool,pub verification:Option<NodeVerificationSnapshot>,pub policy_offset:usize,pub policy:Option<[u8;12]>}
#[derive(Clone,Copy,PartialEq,Eq)] pub(super) enum LinkState {Unknown,Down,Up}
#[derive(Clone,Copy)] pub(super) enum SetupMode {Automatic,Wired,Wireless,Offline}
#[derive(Clone,Copy)] pub(super) enum PolicyAction {Deny,Allow,Ask}
#[derive(Clone,Copy)] pub(super) struct NetworkStatusSnapshot {pub active_profile:u32,pub link_only:bool,pub interfaces:u8,pub addresses:u8,pub routes:u8,pub policies:u8}
#[derive(Clone,Copy)] pub(super) struct NetworkSetupSnapshot {pub selected:SetupMode,pub wired_available:bool,pub wireless_available:bool}
#[derive(Clone,Copy)] pub(super) struct NetworkInterfaceSnapshot {pub enabled:bool,pub link:LinkState,pub hardware_address:bool,pub maximum_frame_size:u64}
#[derive(Clone,Copy)] pub(super) struct AddressSnapshot {pub address:Option<[u8;4]>,pub prefix_length:u8}
#[derive(Clone,Copy)] pub(super) struct RouteSnapshot {pub next_hop:Option<[u8;4]>,pub metric:u32,pub is_static:bool}
#[derive(Clone,Copy)] pub(super) struct NetworkDashboardSnapshot {pub status:NetworkStatusSnapshot,pub active_connections:u64,pub setup:NetworkSetupSnapshot,pub interface:Option<NetworkInterfaceSnapshot>,pub static_address:Option<AddressSnapshot>,pub default_route:Option<RouteSnapshot>,pub resolver_enabled:bool,pub resolver_primary:Option<[u8;4]>,pub resolver_secondary:Option<[u8;4]>,pub default_policy:PolicyAction}
#[derive(Clone,Copy)] pub(super) enum DashboardSnapshot {Network(NetworkDashboardSnapshot),Nodes(NodeDashboardSnapshot)}
impl DisplayDevice {
    // ------------------------=
    // FUNC: render_dashboard_settings_view
    // DESC: Shares production chrome, clipping and dashboard painters with an explicit read-only native preview snapshot.
    // ------------------=
    pub(super) fn render_dashboard_settings_view(&mut self,section:usize,input:&[u8],
        state:crate::ui::system_layout::SettingsWindowState,scale:usize,snapshot:DashboardSnapshot)->bool {
        if !matches!((section,snapshot),(6,DashboardSnapshot::Network(_))|(7,DashboardSnapshot::Nodes(_))) {return false;}
        let old=self.render_clip;
        let chrome=self.paint_settings_chrome(section,state,scale);
        let g=chrome.geometry;
        self.intersect_render_clip(g.viewport.x.max(0)as usize,g.viewport.y.max(0)as usize,
            g.viewport.width as usize,g.viewport.height as usize);
        match snapshot {
            DashboardSnapshot::Network(value)=>self.paint_network_settings_dashboard(state,scale,b"Routed",input,value),
            DashboardSnapshot::Nodes(value)=>self.paint_node_settings_dashboard(state,scale,input,value),
        }
        self.render_clip=old;
        self.render_settings_overflow_chrome(g,state,scale,chrome.outline);
        true
    }
    // ------------------------=
    // FUNC: paint_node_settings_dashboard
    // DESC: Renders the five Milestone 9 node trust, pairing, mesh, policy, and audit operator surfaces from typed state.
    // ------------------=
    pub(super) fn paint_node_settings_dashboard(
        &mut self,
        settings_window: crate::ui::system_layout::SettingsWindowState,
        scale: usize,
        input: &[u8],
        snapshot: NodeDashboardSnapshot,
    ) {
        let layout = crate::ui::system_layout::SystemLayout::new(self.width, self.height);
        let geometry = layout.node_settings_geometry(settings_window);
        let NodeDashboardSnapshot{identity_ready,counts:[discovered,trusted,online,pairings,members,sessions,grants,audit],selected,verification,policy_offset,policy}=snapshot;
        let (outline_r, outline_g, outline_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
        let (selection_r, selection_g, selection_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Selection);
        for (card, role) in [
            (geometry.summary, crate::ui::installer_template::InstallerTemplateRole::SettingsSummaryCard),
            (geometry.main, crate::ui::installer_template::InstallerTemplateRole::SettingsMainCard),
            (geometry.sidebar, crate::ui::installer_template::InstallerTemplateRole::SettingsSidebarCard),
        ] {
            let authored = crate::ui::settings_template::element(7, role);
            self.settings_template_surface(card,authored,crate::ui::panel_recipe::Material{
                fill:(4,18,31),edge:(outline_r/2,outline_g/2,outline_b/2),opacity:224,
                edge_opacity:255,focus:0,pressed:false,scale});
        }
        let page = settings_window.expanded_row.unwrap_or(0).min(4);
        let tabs: [&[u8]; 5] = [
            b"TRUSTED NODES",
            b"PAIR NODE",
            b"MESH HEALTH",
            b"ACCESS POLICY",
            b"SECURITY AUDIT",
        ];
        for index in 0..5 {
            let tab = geometry.tabs[index];
            let active = page == index;
            let authored = crate::ui::settings_template::role_at(
                7, crate::ui::installer_template::InstallerTemplateRole::SettingsTab, index,
            );
            self.settings_template_surface(tab,authored,crate::ui::panel_recipe::Material{
                fill:(5,20,34),edge:(outline_r/2,outline_g/2,outline_b/2),opacity:228,
                edge_opacity:255,focus:if active {255}else{0},pressed:false,scale});
            let label = authored.map(|value| value.text).filter(|text| !text.is_empty()).unwrap_or(tabs[index]);
            let available = (tab.width as usize).saturating_sub(24 * scale);
            let text_width = self.ui_text_width_weighted(label, 1, true).min(available);
            self.ui_text_elided_strong(
                tab.x.max(0) as usize + (tab.width as usize - text_width) / 2,
                tab.y.max(0) as usize + (tab.height as usize).saturating_sub(UI_FONT_CELL_HEIGHT) / 2,
                available,
                label,
                if active { 242 } else { 166 },
                if active { 248 } else { 190 },
                if active { 252 } else { 207 },
            );
        }
        let summary_left = geometry.summary.x.max(0) as usize;
        let summary_top = geometry.summary.y.max(0) as usize;
        self.authentication_icon(
            summary_left + 36 * scale,
            summary_top + geometry.summary.height as usize / 2,
            6,
            44 * scale,
            true,
        );
        self.ui_text_strong(
            summary_left + 74 * scale,
            summary_top + 14 * scale,
            crate::ui::settings_template::element(
                7, crate::ui::installer_template::InstallerTemplateRole::SettingsSummaryCard,
            ).map(|value| value.text).filter(|text| !text.is_empty()).unwrap_or(b"NODE IDENTITY"),
            outline_r,
            outline_g,
            outline_b,
            1,
        );
        self.ui_text_elided_strong(
            summary_left + 74 * scale,
            summary_top + 46 * scale,
            (geometry.summary.width as usize).saturating_sub(96 * scale),
            if identity_ready {
                b"Cryptographic identity ready"
            } else {
                b"Identity state refreshing / unavailable"
            },
            239,
            246,
            251,
        );
        let mut labels: [[&[u8]; 2]; 6] = match page {
            0 => [
                [b"DISCOVERED", b"Observed, not trusted"],
                [b"TRUSTED", b"Explicit relationships"],
                [b"ONLINE", b"Authenticated reachability"],
                [b"SESSIONS", b"Mutually authenticated"],
                [b"AUTHORITY", b"Capability scoped"],
                [b"REFRESH", b"Discover local nodes"],
            ],
            1 => [
                [b"SELECT NODE", b"Choose an untrusted peer"],
                [b"VERIFY IDENTITY", b"Compare fingerprint"],
                [b"PAIRING CODE", b"Confirm on both nodes"],
                [b"CONFIRM", b"Trusted UI required"],
                [b"CANCEL", b"Grant no authority"],
                [b"PAIRING STATE", b"Short lived transaction"],
            ],
            2 => [
                [b"MEMBERS", b"Explicit domain membership"],
                [b"ONLINE", b"Recent authenticated heartbeat"],
                [b"DEGRADED", b"Partial availability"],
                [b"OFFLINE", b"State retained safely"],
                [b"LEAVE DOMAIN", b"Preserve node trust"],
                [b"DIAGNOSTICS", b"Health and compatibility"],
            ],
            3 => [
                [b"OBJECT ACCESS", b"Deny by default"],
                [b"REMOTE OPERATIONS", b"Scoped capabilities"],
                [b"AI CONTEXT", b"No ambient sharing"],
                [b"RESOURCE USE", b"Separate milestone"],
                [b"LEASE", b"Expiry enforced"],
                [b"COMMIT POLICY", b"Persist then announce"],
            ],
            _ => [
                [b"TRUST EVENTS", b"Structured records"],
                [b"PAIRING EVENTS", b"Correlation preserved"],
                [b"SESSION EVENTS", b"No secret material"],
                [b"POLICY EVENTS", b"Auditable changes"],
                [b"EXPORT", b"Typed projection"],
                [b"REFRESH", b"Read durable records"],
            ],
        };
        let values = [
            discovered,
            trusted,
            online,
            sessions,
            grants,
            if page == 1 {
                pairings
            } else if page == 2 {
                members
            } else {
                audit
            },
        ];
        if page == 3 {
            let categories: [&[u8]; 12] = [b"OBJECT", b"NAMESPACE", b"COMPUTE", b"AI", b"SERVICE", b"EVENT", b"STORAGE", b"CLIPBOARD", b"DEVICE", b"DIAGNOSTICS", b"MESH", b"ADMINISTRATIVE"];
            for index in 0..5 {
                labels[index] = if policy_offset + index < 12 { [categories[policy_offset + index], b"Deny / session / allow"] } else { [b"", b""] };
            }
            labels[5] = [b"NEXT CATEGORIES", b"Twelve independent policies"];
        }
        for index in 0..6 {
            let card = geometry.controls[index];
            let active = settings_window.control_focus.min(5) == index;
            let authored = crate::ui::settings_template::role_at(
                7, crate::ui::installer_template::InstallerTemplateRole::Metadata, index,
            );
            self.settings_template_surface(card,authored,crate::ui::panel_recipe::Material{
                fill:(6,24,39),edge:(outline_r/2,outline_g/2,outline_b/2),opacity:230,
                edge_opacity:255,focus:if active {255}else{0},pressed:false,scale});
            let (mut number, mut length) = Self::network_metric_text(values[index] as u64);
            if page == 1 {
                let value: &[u8] = match index {
                    0 => if selected { b"Selected" } else { b"Choose" },
                    1 => if verification.is_some() { b"Ready" } else { b"Begin" },
                    2 => if verification.is_some() { b"Compare" } else { b"Waiting" },
                    3 => b"Enter code",
                    4 => b"Cancel",
                    _ => match verification.map(|value| value.state) {
                        Some(VerificationState::Local) => b"Peer pending",
                        Some(VerificationState::Remote) => b"Your turn",
                        Some(VerificationState::Confirmed) => b"Confirmed",
                        Some(_) => b"Verify",
                        None => b"No transcript",
                    },
                };
                length = value.len().min(number.len()); number[..length].copy_from_slice(&value[..length]);
            }
            if page == 3 {
                let value: &[u8] = if index == 5 { match policy_offset { 0 => b"1 / 3", 5 => b"2 / 3", _ => b"3 / 3" } }
                else if policy_offset + index >= 12 { b"" }
                else { match policy.map(|record| record[policy_offset + index]) { Some(0) => b"Deny", Some(1) => b"Allow", Some(2) => b"Session", Some(3) => b"Leased", _ => b"Select peer" } };
                length = value.len().min(number.len()); number[..length].copy_from_slice(&value[..length]);
            }
            let number_width = self.ui_text_width(&number[..length], 1);
            let label_width = (card.width as usize).saturating_sub(number_width + 48 * scale);
            self.ui_text_elided_strong(card.x.max(0) as usize + 15 * scale,
                card.y.max(0) as usize + 12 * scale, label_width, labels[index][0], 220, 239, 249);
            self.ui_text_elided_strong(card.x.max(0) as usize + 15 * scale,
                card.y.max(0) as usize + 44 * scale,
                (card.width as usize).saturating_sub(30 * scale), labels[index][1], 145, 174, 193);
            self.ui_text(
                card.right().max(0) as usize - number_width - 14 * scale,
                card.y.max(0) as usize + 18 * scale,
                &number[..length],
                outline_r,
                outline_g,
                outline_b,
                1,
            );
        }
        let art = geometry.sidebar;
        if page == 1 {
            // Verification is live authenticated state, never decorative artwork.
            let left = art.x.max(0) as usize + 15 * scale;
            let top = art.y.max(0) as usize + 14 * scale;
            let width = (art.width as usize).saturating_sub(30 * scale);
            self.ui_text_wrapped(left, top, width, b"COMPARE BOTH SCREENS", 220, 239, 249, 2);
            if let Some(verification) = verification {
                let mut code = [b'0'; 6]; let mut value = verification.code;
                for digit in code.iter_mut().rev() { *digit += (value % 10) as u8; value /= 10; }
                self.ui_text_fit_strong(left, top + 72 * scale, width, &code, outline_r, outline_g, outline_b, 2);
                self.ui_text(left, top + 144 * scale, b"Transcript fingerprint", 145, 174, 193, 1);
                let digits = b"0123456789abcdef";
                for row in 0..4 {
                    let mut line = [0; 16];
                    for column in 0..8 { let byte = verification.fingerprint[row * 8 + column]; line[column * 2] = digits[(byte >> 4) as usize]; line[column * 2 + 1] = digits[(byte & 15) as usize]; }
                    self.ui_text_fit_strong(left, top + (176 + row * 32) * scale, width, &line, 220, 239, 249, 1);
                }
                let remaining = verification.remaining;
                let (value, length) = Self::network_metric_text(remaining);
                self.ui_text(left, top + 328 * scale, b"Seconds remaining", 145, 174, 193, 1);
                self.ui_text(left, top + 360 * scale, &value[..length], outline_r, outline_g, outline_b, 1);
                self.ui_text(left, top + 408 * scale, b"Enter peer code", 145, 174, 193, 1);
                self.ui_text_fit_strong(left, top + 440 * scale, width, &input[..input.len().min(6)], 242, 248, 252, 2);
                self.ui_text(left, top + 520 * scale, b"Enter: confirm", 145, 174, 193, 1);
                self.ui_text(left, top + 552 * scale, b"Esc: cancel", 145, 174, 193, 1);
            } else {
                self.ui_text_wrapped(left, top + 42 * scale, width, b"Select a peer and begin pairing. No authority is granted until both operators confirm the matching fingerprint and code.", 145, 174, 193, 10);
            }
            return;
        }
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        {
            let template = crate::ui::settings_template::template();
            let authored = crate::ui::settings_template::element(
                7, crate::ui::installer_template::InstallerTemplateRole::SettingsArtwork,
            );
            let authored_rect = art;
            let bytes = authored
                .and_then(|element| template.and_then(|value| value.asset(element.image_asset)))
                .unwrap_or(super::desktop::NODE_TRUST_TOPOLOGY_BMP);
            let available_width = (authored_rect.width as usize).saturating_sub(24 * scale);
            let available_height = (authored_rect.height as usize).saturating_sub(144 * scale);
            let source_width = if bytes.len() >= 54 { super::primitives::le32(bytes, 18) as usize } else { 1 }.max(1);
            let source_height = if bytes.len() >= 54 { (super::primitives::le32(bytes, 22) as i32).unsigned_abs() as usize } else { 1 }.max(1);
            let image_width = available_width.min(available_height * source_width / source_height);
            let image_height = image_width * source_height / source_width;
            self.paint_bitmap_template_rect(
                bytes,
                authored_rect.x.max(0) as usize + (authored_rect.width as usize - image_width) / 2,
                authored_rect.y.max(0) as usize + 12 * scale,
                image_width,
                image_height,
                authored.map(|element|element.crop).unwrap_or([0;4]),
                authored.map(|element|if element.hidden{0}else{(u16::from(element.opacity)*255/100)as u8}).unwrap_or(255),
                false,
            );
        }
        #[cfg(target_arch = "x86")]
        self.fill_rect(
            art.x.max(0) as usize + 2 * scale,
            art.y.max(0) as usize + 2 * scale,
            (art.width as usize).saturating_sub(4 * scale),
            (art.height as usize).saturating_sub(4 * scale),
            2,
            13,
            25,
        );
        self.fill_rect_alpha(
            art.x.max(0) as usize + 2 * scale,
            art.bottom().max(0) as usize - 70 * scale,
            (art.width as usize).saturating_sub(4 * scale),
            68 * scale,
            1,
            11,
            22,
            220,
        );
        self.ui_text_wrapped(art.x.max(0) as usize + 16 * scale,
            art.bottom().max(0) as usize - 132 * scale,
            (art.width as usize).saturating_sub(32 * scale),
            b"Discovery is not trust. Trust is not authority.", 185, 215, 233, 4);
    }

    // ------------------------=
    // FUNC: paint_network_settings_dashboard
    // DESC: Renders a scalable live network overview, topology, telemetry, and operational profile selector.
    // ------------------=
    pub(super) fn paint_network_settings_dashboard(
        &mut self,
        settings_window: crate::ui::system_layout::SettingsWindowState,
        scale: usize,
        connectivity: &[u8],
        input: &[u8],
        snapshot: NetworkDashboardSnapshot,
    ) {
        let layout = crate::ui::system_layout::SystemLayout::new(self.width, self.height);
        let geometry = layout.network_settings_geometry(settings_window);
        let NetworkDashboardSnapshot{status,active_connections,setup,interface,static_address,default_route,resolver_enabled,resolver_primary,resolver_secondary,default_policy}=snapshot;
        let connectivity: &[u8] = match interface {
            Some(value) if !value.enabled => b"Adapter disabled",
            Some(_) if status.active_profile == 3 => b"Offline profile",
            Some(value) if value.link == LinkState::Unknown => b"Link not verified",
            Some(value) if value.link == LinkState::Down => b"Link disconnected",
            Some(_) if status.link_only => b"Link up, no address",
            _ => connectivity,
        };
        let (outline_r, outline_g, outline_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::WindowOutline);
        let (selection_r, selection_g, selection_b) =
            self.active_accent_surface(crate::ui::skin::AccentSurface::Selection);
        let cards = [
            (geometry.summary, crate::ui::installer_template::InstallerTemplateRole::SettingsSummaryCard),
            (geometry.main, crate::ui::installer_template::InstallerTemplateRole::SettingsMainCard),
            (geometry.sidebar, crate::ui::installer_template::InstallerTemplateRole::SettingsSidebarCard),
        ];
        for (card, role) in cards {
            let authored = crate::ui::settings_template::element(6, role);
            self.settings_template_surface(card,authored,crate::ui::panel_recipe::Material{
                fill:(4,18,31),edge:(outline_r/2,outline_g/2,outline_b/2),opacity:220,
                edge_opacity:255,focus:0,pressed:false,scale});
        }

        let page = settings_window.expanded_row.unwrap_or(0).min(6);
        let page_labels: [&[u8]; 7] = [
            b"OVERVIEW",
            b"INTERFACES",
            b"IPv4",
            b"DNS",
            b"ROUTES",
            b"PROFILES",
            b"POLICY",
        ];
        for (index, tab) in geometry.tabs.iter().enumerate() {
            let active = page == index;
            let authored = crate::ui::settings_template::role_at(
                6, crate::ui::installer_template::InstallerTemplateRole::SettingsTab, index,
            );
            self.settings_template_surface(*tab,authored,crate::ui::panel_recipe::Material{
                fill:(5,20,34),edge:(outline_r/2,outline_g/2,outline_b/2),opacity:226,
                edge_opacity:255,focus:if active {255}else{0},pressed:false,scale});
            let label = authored.map(|value| value.text).filter(|text| !text.is_empty()).unwrap_or(page_labels[index]);
            let available = (tab.width as usize).saturating_sub(24 * scale);
            let text_width = self.ui_text_width_weighted(label, 1, true).min(available);
            self.ui_text_elided_strong(
                tab.x.max(0) as usize + (tab.width as usize - text_width) / 2,
                tab.y.max(0) as usize + (tab.height as usize).saturating_sub(UI_FONT_CELL_HEIGHT) / 2,
                available,
                label,
                if active { 242 } else { 166 },
                if active { 248 } else { 190 },
                if active { 252 } else { 207 },
            );
        }

        let overview_left = geometry.summary.x.max(0) as usize;
        let overview_top = geometry.summary.y.max(0) as usize;
        self.authentication_icon(
            overview_left + 36 * scale,
            overview_top + geometry.summary.height as usize / 2,
            14,
            44 * scale,
            true,
        );
        self.ui_text_strong(
            overview_left + 74 * scale,
            overview_top + 14 * scale,
            crate::ui::settings_template::element(
                6, crate::ui::installer_template::InstallerTemplateRole::SettingsSummaryCard,
            ).map(|value| value.text).filter(|text| !text.is_empty()).unwrap_or(b"CONNECTIVITY"),
            outline_r,
            outline_g,
            outline_b,
            1,
        );
        self.ui_text_elided_strong(
            overview_left + 74 * scale,
            overview_top + 39 * scale,
            (geometry.summary.width as usize).saturating_sub(252 * scale),
            connectivity,
            239,
            246,
            251,
        );
        let profile_name: &[u8] = match status.active_profile {
            1 => b"STANDARD",
            2 => b"RESTRICTED",
            3 => b"OFFLINE",
            4 => b"OPERATIONS",
            5 => b"DEVELOPER",
            _ => b"CUSTOM",
        };
        let pill_width = 142 * scale;
        let pill_left = overview_left + geometry.summary.width as usize - pill_width - 18 * scale;
        self.fill_rounded_rect_alpha(
            pill_left,
            overview_top + 17 * scale,
            pill_width,
            48 * scale,
            24 * scale,
            selection_r,
            selection_g,
            selection_b,
            220,
        );
        self.ui_text_centered(
            pill_left,
            pill_width,
            overview_top + 33 * scale,
            profile_name,
            242,
            249,
            253,
            1,
        );

        let mode_labels: [&[u8]; 4] = [b"AUTOMATIC", b"WIRED", b"WI-FI", b"OFFLINE"];
        let profile_labels: [&[u8]; 5] = [
            b"STANDARD",
            b"RESTRICTED",
            b"OFFLINE",
            b"OPERATIONS",
            b"DEVELOPER",
        ];
        let control_labels: [&[u8]; 6] = match page {
            0 => [
                mode_labels[0],
                mode_labels[1],
                mode_labels[2],
                mode_labels[3],
                b"REFRESH STATE",
                b"OPEN DIAGNOSTICS",
            ],
            1 => [
                b"PRIMARY ADAPTER",
                b"ENABLED",
                b"LINK STATE",
                b"HARDWARE ADDRESS",
                b"MAXIMUM FRAME",
                b"REFRESH DEVICES",
            ],
            2 => [
                b"ADDRESSING MODE",
                b"IPv4 ADDRESS",
                b"PREFIX LENGTH",
                b"DEFAULT GATEWAY",
                b"ROUTE METRIC",
                b"APPLY STATIC CONFIG",
            ],
            3 => [
                b"RESOLVER",
                b"PRIMARY DNS",
                b"SECONDARY DNS",
                b"CLEAR DNS SERVERS",
                b"CACHE ENTRIES",
                b"APPLY DNS CONFIG",
            ],
            4 => [
                b"DEFAULT ROUTE",
                b"NEXT HOP",
                b"INTERFACE",
                b"METRIC",
                b"ROUTE SOURCE",
                b"REMOVE DEFAULT ROUTE",
            ],
            5 => [
                profile_labels[0],
                profile_labels[1],
                profile_labels[2],
                profile_labels[3],
                profile_labels[4],
                b"RESTORE STANDARD",
            ],
            _ => [
                b"UNMATCHED OUTBOUND",
                b"LOCAL DISCOVERY",
                b"INBOUND LISTENERS",
                b"AUDIT DECISIONS",
                b"INSTALLED RULES",
                b"RESTORE SAFE POLICY",
            ],
        };
        let (address_text, address_length) =
            Self::network_address_text(static_address.and_then(|value| value.address));
        let (gateway_text, gateway_length) =
            Self::network_address_text(default_route.and_then(|value| value.next_hop));
        let (primary_text, primary_length) = Self::network_address_text(resolver_primary);
        let (secondary_text, secondary_length) = Self::network_address_text(resolver_secondary);
        let (prefix_text, prefix_length) = Self::network_metric_text(
            static_address
                .map(|value| value.prefix_length as u64)
                .unwrap_or(24),
        );
        let (metric_text, metric_length) = Self::network_metric_text(
            default_route
                .map(|value| value.metric as u64)
                .unwrap_or(100),
        );
        let (mtu_text, mtu_length) = Self::network_metric_text(
            interface
                .map(|value| value.maximum_frame_size as u64)
                .unwrap_or(0),
        );
        let active_profile = status.active_profile.saturating_sub(1) as usize;
        for (index, card) in geometry.controls.iter().enumerate() {
            let left = card.x.max(0) as usize;
            let top = card.y.max(0) as usize;
            let active = settings_window.control_focus.min(5) == index;
            let authored = crate::ui::settings_template::role_at(
                6, crate::ui::installer_template::InstallerTemplateRole::Metadata, index,
            );
            self.settings_template_surface(*card,authored,crate::ui::panel_recipe::Material{
                fill:(6,24,39),edge:(outline_r/2,outline_g/2,outline_b/2),opacity:230,
                edge_opacity:255,focus:if active {255}else{0},pressed:false,scale});
            self.ui_text_strong(
                left + 15 * scale,
                top + card.height as usize / 2 - UI_FONT_CELL_HEIGHT / 2,
                control_labels[index],
                if active { 240 } else { 190 },
                if active { 247 } else { 211 },
                if active { 251 } else { 224 },
                1,
            );
            let value: &[u8] = match page {
                0 if index < 4 => {
                    let selected = index
                        == match setup.selected {
                            SetupMode::Automatic => 0,
                            SetupMode::Wired => 1,
                            SetupMode::Wireless => 2,
                            SetupMode::Offline => 3,
                        };
                    if selected {
                        b"ACTIVE"
                    } else {
                        b"SELECT"
                    }
                }
                0 => b"RUN",
                1 => match index {
                    0 => {
                        if interface.is_some() {
                            b"network0"
                        } else {
                            b"Not detected"
                        }
                    }
                    1 => {
                        if interface.map(|value| value.enabled).unwrap_or(false) {
                            b"ON"
                        } else {
                            b"OFF"
                        }
                    }
                    2 => match interface.map(|value| value.link) {
                        Some(LinkState::Up) => b"UP",
                        Some(LinkState::Down) => b"DOWN",
                        _ => b"UNKNOWN",
                    },
                    3 => {
                        if interface
                            .map(|value| value.hardware_address)
                            .unwrap_or(false)
                        {
                            b"Observed"
                        } else {
                            b"Unavailable"
                        }
                    }
                    4 => &mtu_text[..mtu_length],
                    _ => b"RUN",
                },
                2 => match index {
                    0 => {
                        if static_address.is_some() {
                            b"STATIC"
                        } else {
                            b"DYNAMIC"
                        }
                    }
                    1 => &address_text[..address_length],
                    2 => &prefix_text[..prefix_length],
                    3 => &gateway_text[..gateway_length],
                    4 => &metric_text[..metric_length],
                    _ => b"COMMIT",
                },
                3 => match index {
                    0 => {
                        if resolver_enabled {
                            b"ON"
                        } else {
                            b"OFF"
                        }
                    }
                    1 => &primary_text[..primary_length],
                    2 => &secondary_text[..secondary_length],
                    3 => b"CLEAR",
                    4 => b"BOUNDED",
                    _ => b"COMMIT",
                },
                4 => match index {
                    0 => {
                        if default_route.is_some() {
                            b"INSTALLED"
                        } else {
                            b"NONE"
                        }
                    }
                    1 => &gateway_text[..gateway_length],
                    2 => b"network0",
                    3 => &metric_text[..metric_length],
                    4 => {
                        if default_route
                            .map(|value| {
                                value.is_static
                            })
                            .unwrap_or(false)
                        {
                            b"STATIC"
                        } else {
                            b"DISCOVERED"
                        }
                    }
                    _ => b"REMOVE",
                },
                5 if index < 5 => {
                    if active_profile == index {
                        b"ACTIVE"
                    } else {
                        b"SELECT"
                    }
                }
                5 => b"RESTORE",
                _ => match index {
                    0 => match default_policy {
                        PolicyAction::Allow => b"ALLOW",
                        PolicyAction::Ask => b"ASK",
                        _ => b"DENY",
                    },
                    1 => b"PROFILE CONTROLLED",
                    2 => b"PROFILE CONTROLLED",
                    3 => b"ON",
                    4 => b"INSPECT",
                    _ => b"SAFE DEFAULTS",
                },
            };
            let displayed = if active
                && !input.is_empty()
                && matches!((page, index), (2, 1..=4) | (3, 1..=2))
            {
                input
            } else {
                value
            };
            let value_width = self.ui_text_width(displayed, 1);
            self.ui_text(
                left + card.width as usize - value_width - 18 * scale,
                top + card.height as usize / 2 - UI_FONT_CELL_HEIGHT / 2,
                displayed,
                outline_r,
                outline_g,
                outline_b,
                1,
            );
            if active && matches!((page, index), (2, 1..=4) | (3, 1..=2)) {
                self.text_field_caret(
                    left + card.width as usize - value_width - 18 * scale,
                    top,
                    card.height as usize,
                    input,
                    true,
                    1,
                );
            }
        }

        let sidebar_left = geometry.sidebar.x.max(0) as usize;
        let sidebar_top = geometry.sidebar.y.max(0) as usize;
        self.ui_text_strong(
            sidebar_left + 17 * scale,
            sidebar_top + 16 * scale,
            b"LIVE CONFIGURATION",
            outline_r,
            outline_g,
            outline_b,
            1,
        );
        let metrics = [
            (b"Interfaces".as_slice(), status.interfaces as u64),
            (b"Addresses".as_slice(), status.addresses as u64),
            (b"Routes".as_slice(), status.routes as u64),
            (
                b"Connections".as_slice(),
                active_connections,
            ),
            (b"Policy rules".as_slice(), status.policies as u64),
        ];
        for (index, (label, value)) in metrics.iter().enumerate() {
            let row_y = sidebar_top + (48 + index * 30) * scale;
            self.ui_text(sidebar_left + 17 * scale, row_y, label, 169, 190, 206, 1);
            let (digits, length) = Self::network_metric_text(*value);
            self.ui_text_strong(
                sidebar_left + geometry.sidebar.width as usize - (28 + length * 10) * scale,
                row_y,
                &digits[..length],
                235,
                245,
                251,
                1,
            );
        }

        let detail_top = sidebar_top + 220 * scale;
        self.ui_text_strong(
            sidebar_left + 17 * scale,
            detail_top,
            page_labels[page],
            outline_r,
            outline_g,
            outline_b,
            1,
        );
        let state_lines: [&[u8]; 5] = match page {
            0 => [
                b"Choose a connection mode.",
                b"Changes apply transactionally.",
                if setup.wired_available {
                    b"Wired adapter available"
                } else {
                    b"No wired adapter"
                },
                if setup.wireless_available {
                    b"Wi-Fi adapter available"
                } else {
                    b"No Wi-Fi adapter"
                },
                b"Local services remain available offline.",
            ],
            1 => [
                if interface.is_some() {
                    b"Adapter discovered"
                } else {
                    b"No external adapter"
                },
                if interface.map(|value| value.enabled).unwrap_or(false) {
                    b"Interface enabled"
                } else {
                    b"Interface disabled"
                },
                b"Hardware values are observed only.",
                b"Toggle the adapter with ENABLED.",
                b"Refresh never fabricates link state.",
            ],
            2 => [
                if static_address.is_some() {
                    b"Static IPv4 is configured."
                } else {
                    b"Dynamic addressing is active."
                },
                b"Address format: 10.0.2.15",
                b"Prefix range: 0 through 32",
                b"Gateway is optional.",
                b"Apply commits address and route.",
            ],
            3 => [
                if resolver_enabled {
                    b"Resolver enabled"
                } else {
                    b"Resolver disabled"
                },
                if resolver_primary.is_some() {
                    b"Primary server configured"
                } else {
                    b"No primary server configured"
                },
                if resolver_secondary.is_some() {
                    b"Secondary server configured"
                } else {
                    b"Secondary server not set"
                },
                b"Cache and queries are bounded.",
                b"Wire DNS depends on adapter support.",
            ],
            4 => [
                if default_route.is_some() {
                    b"Default route installed"
                } else {
                    b"No default route"
                },
                b"Longest-prefix selection is active.",
                b"Lower metric wins ties.",
                b"Static routes persist across boot.",
                b"Route changes are capability gated.",
            ],
            5 => [
                b"Standard: ordinary connectivity",
                b"Restricted: local-first policy",
                b"Offline: disables external access",
                b"Operations: controlled inbound",
                b"Developer: broader local services",
            ],
            _ => [
                match default_policy {
                    PolicyAction::Allow => b"Default: allow",
                    PolicyAction::Ask => b"Default: ask",
                    _ => b"Default: deny",
                },
                b"Rules bind stable identities.",
                b"No executable-path authority.",
                b"Changes persist as typed state.",
                b"Unknown applications remain denied.",
            ],
        };
        let mut line_top = detail_top + 36 * scale;
        let line_width = (geometry.sidebar.width as usize).saturating_sub(34 * scale);
        for line in state_lines.iter() {
            let lines = self.ui_text_wrapped_line_count(line_width, line, 3).max(1);
            self.ui_text_wrapped(
                sidebar_left + 17 * scale,
                line_top,
                line_width,
                line,
                174,
                198,
                215,
                3,
            );
            line_top += lines * (UI_FONT_CELL_HEIGHT + 4) * self.ui_scale() + 8 * scale;
        }
    }

    // ------------------------=
    // FUNC: network_metric_text
    // DESC: Formats one bounded live network count without allocation or fabricated units.
    // ------------------=
    fn network_metric_text(mut value: u64) -> ([u8; 20], usize) {
        let mut output = [b'0'; 20];
        if value == 0 {
            return (output, 1);
        }
        let mut reverse = [0u8; 20];
        let mut length = 0usize;
        while value > 0 && length < reverse.len() {
            reverse[length] = b'0' + (value % 10) as u8;
            value /= 10;
            length += 1;
        }
        for index in 0..length {
            output[index] = reverse[length - index - 1];
        }
        (output, length)
    }

    // ------------------------=
    // FUNC: network_address_text
    // DESC: Formats an optional typed IPv4 address for the Settings projection without allocating text state.
    // ------------------=
    fn network_address_text(
        address: Option<[u8;4]>,
    ) -> ([u8; 15], usize) {
        let mut output = [0u8; 15];
        let Some(octets) = address else {
            output[..9].copy_from_slice(b"Automatic");
            return (output, 9);
        };
        let mut length = 0usize;
        for (index, octet) in octets.iter().copied().enumerate() {
            if index != 0 {
                output[length] = b'.';
                length += 1;
            }
            let hundreds = octet / 100;
            let tens = (octet / 10) % 10;
            if hundreds != 0 {
                output[length] = b'0' + hundreds;
                length += 1;
                output[length] = b'0' + tens;
                length += 1;
            } else if tens != 0 {
                output[length] = b'0' + tens;
                length += 1;
            }
            output[length] = b'0' + octet % 10;
            length += 1;
        }
        (output, length)
    }

}
