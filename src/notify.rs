//! notify.rs — Notificaciones de escritorio multiplataforma.
//!
//! Batch Image Generator — Eric Valls Gramunt
//!
//! Se apoya en `notify-rust`, que cubre los tres sistemas con una sola API:
//!
//!   • **Linux** — especificación XDG por D-Bus. Funciona igual en Wayland y
//!     en XWayland, porque el transporte es D-Bus y no el servidor gráfico:
//!     habla con el demonio de notificaciones del escritorio (GNOME Shell,
//!     KDE, mako, dunst…), no con X11 ni con el compositor.
//!   • **Windows 10/11** — toasts WinRT.
//!   • **macOS** — `UNUserNotificationCenter`.
//!
//! DOS REGLAS DE ORO
//! -----------------
//! 1. **Nunca bloquear la interfaz.** `show()` hace E/S —D-Bus en Linux— y
//!    puede tardar o fallar. Cada aviso sale en su propio hilo.
//! 2. **Nunca tumbar la aplicación.** Si no hay demonio de notificaciones, o
//!    el usuario las tiene desactivadas en el sistema, o falla el permiso en
//!    macOS, el error se anota y se sigue generando. Una notificación es un
//!    extra, jamás un requisito.
//!
//! Se usa a propósito el subconjunto de la API que existe en las tres
//! plataformas (`summary`, `body`, `appname`). Métodos como `timeout()`,
//! `hint()` o `urgency()` sólo están en algunas y obligarían a compilación
//! condicional sin aportar nada aquí.

/// Qué acaba de pasar. Determina el título y si el aviso llega a salir.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// La imagen se generó y se guardó.
    Success,
    /// El servidor agotó su tiempo o no respondió.
    Timeout,
    /// El proveedor rechazó el prompt o la imagen por sus políticas.
    ContentPolicy,
    /// Cualquier otro fallo.
    OtherError,
}

/// Preferencias del usuario. Cada tipo de aviso se activa por separado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Interruptor general: en `false` no sale ningún aviso.
    pub enabled: bool,
    pub on_success: bool,
    pub on_timeout: bool,
    pub on_policy: bool,
    pub on_other_error: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            // Los fallos vienen activados y el éxito no: en un Burst largo
            // avisar de cada acierto son cientos de notificaciones, mientras
            // que de los fallos sí interesa enterarse en el momento.
            on_success: false,
            on_timeout: true,
            on_policy: true,
            on_other_error: true,
        }
    }
}

impl Settings {
    pub fn should_notify(&self, event: Event) -> bool {
        if !self.enabled {
            return false;
        }
        match event {
            Event::Success => self.on_success,
            Event::Timeout => self.on_timeout,
            Event::ContentPolicy => self.on_policy,
            Event::OtherError => self.on_other_error,
        }
    }
}

/// Clasifica el mensaje de error de un proveedor.
///
/// Los proveedores no devuelven un código uniforme para esto, así que hay que
/// mirar el texto. Los patrones salen de mensajes reales observados:
///
/// ```text
/// WaveSpeed error: Content flagged as potentially sensitive. …
/// Kie.AI falló tras 90s (código 524): generate task timeout.
/// Timeout: WaveSpeed tardó demasiado.
/// ```
pub fn classify(msg: &str) -> Event {
    let m = msg.to_lowercase();

    // Primero políticas de contenido: algunos mensajes de rechazo mencionan
    // también «request» o cifras que podrían confundirse con otra cosa.
    const POLITICA: &[&str] = &[
        "content flagged",
        "potentially sensitive",
        "content policy",
        "content_policy",
        "safety system",
        "safety",
        "moderation",
        "nsfw",
        "inappropriate",
        "not allowed",
        "prohibited",
        "violates",
        "rechaz",
        "sensible",
    ];
    if POLITICA.iter().any(|p| m.contains(p)) {
        return Event::ContentPolicy;
    }

    const TIEMPO: &[&str] = &[
        "timeout",
        "timed out",
        "tardó demasiado",
        "se agotó la espera",
        "agotó su propio tiempo",
        "task timeout",
        "deadline",
    ];
    if TIEMPO.iter().any(|p| m.contains(p)) {
        return Event::Timeout;
    }

    Event::OtherError
}

fn titulo(event: Event) -> &'static str {
    match event {
        Event::Success => "✅ Imagen generada",
        Event::Timeout => "⏱ Tiempo agotado",
        Event::ContentPolicy => "🚫 Rechazado por contenido",
        Event::OtherError => "❌ Error de generación",
    }
}

/// Recorta el cuerpo para que quepa en un aviso del sistema.
///
/// Corta por caracteres, no por bytes: los mensajes llevan acentos y emojis,
/// y `&s[..n]` habría entrado en pánico exactamente igual que en la 2.4.0.
fn cuerpo(msg: &str) -> String {
    let limpio: String = msg.replace('\n', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    crate::util::truncate_chars(&limpio, 180)
}

/// Lanza el aviso si las preferencias lo permiten.
///
/// Devuelve `true` si se ha intentado enviar. No espera al resultado: el
/// envío ocurre en un hilo aparte para no bloquear el bucle de la interfaz.
pub fn notify(settings: Settings, event: Event, msg: &str) -> bool {
    if !settings.should_notify(event) {
        return false;
    }

    let summary = titulo(event);
    let body = cuerpo(msg);

    std::thread::Builder::new()
        .name("notificacion".into())
        .spawn(move || {
            let resultado = notify_rust::Notification::new()
                .summary(summary)
                .body(&body)
                .appname("Batch Image Generator")
                .show();

            if let Err(e) = resultado {
                // Sin demonio de notificaciones, sin permiso en macOS o sin
                // sesión de escritorio. No es motivo para interrumpir nada.
                eprintln!("[notify] no se pudo mostrar el aviso: {e}");
            }
        })
        .map(|_| true)
        .unwrap_or_else(|e| {
            eprintln!("[notify] no se pudo crear el hilo: {e}");
            false
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_interruptor_general_manda_sobre_los_demas() {
        let s = Settings {
            enabled: false,
            on_success: true,
            on_timeout: true,
            on_policy: true,
            on_other_error: true,
        };
        for e in [
            Event::Success,
            Event::Timeout,
            Event::ContentPolicy,
            Event::OtherError,
        ] {
            assert!(!s.should_notify(e), "{e:?} no debería avisar con todo apagado");
        }
    }

    #[test]
    fn cada_tipo_se_activa_por_separado() {
        let s = Settings {
            enabled: true,
            on_success: false,
            on_timeout: true,
            on_policy: false,
            on_other_error: false,
        };
        assert!(!s.should_notify(Event::Success));
        assert!(s.should_notify(Event::Timeout));
        assert!(!s.should_notify(Event::ContentPolicy));
        assert!(!s.should_notify(Event::OtherError));
    }

    /// Por defecto avisa de los fallos pero no de cada acierto: en Burst
    /// serían cientos de notificaciones.
    #[test]
    fn por_defecto_avisa_de_fallos_y_no_de_aciertos() {
        let s = Settings { enabled: true, ..Settings::default() };
        assert!(!s.should_notify(Event::Success));
        assert!(s.should_notify(Event::Timeout));
        assert!(s.should_notify(Event::ContentPolicy));
    }

    /// Mensajes reales observados en ejecución.
    #[test]
    fn clasifica_los_mensajes_reales_de_los_proveedores() {
        assert_eq!(
            classify("WaveSpeed error: Content flagged as potentially sensitive. Please try different prompts or images."),
            Event::ContentPolicy
        );
        assert_eq!(
            classify("⏱ Kie.AI agotó su propio tiempo de generación tras 90s (código 524)."),
            Event::Timeout
        );
        assert_eq!(classify("Timeout: WaveSpeed tardó demasiado."), Event::Timeout);
        assert_eq!(
            classify("Kie.AI: se agotó la espera del resultado tras 300s."),
            Event::Timeout
        );
        assert_eq!(
            classify("WaveSpeed devolvió HTTP 401: invalid api key"),
            Event::OtherError
        );
    }

    /// Un rechazo por contenido que además mencione tiempos no debe
    /// clasificarse como timeout: la política manda.
    #[test]
    fn la_politica_de_contenido_manda_sobre_el_timeout() {
        assert_eq!(
            classify("Content flagged as sensitive after 30s timeout window"),
            Event::ContentPolicy
        );
    }

    #[test]
    fn clasificar_no_distingue_mayusculas() {
        assert_eq!(classify("CONTENT FLAGGED"), Event::ContentPolicy);
        assert_eq!(classify("Generate Task TIMEOUT"), Event::Timeout);
    }

    #[test]
    fn el_cuerpo_se_recorta_sin_romper_caracteres() {
        let largo = format!("{}ñ{}", "a".repeat(179), "b".repeat(200));
        let c = cuerpo(&largo); // no debe entrar en pánico
        assert!(c.chars().count() <= 181);

        // Los saltos de línea se aplanan: un aviso del sistema es una línea.
        assert_eq!(cuerpo("una\nlínea\n\ny  otra"), "una línea y otra");
    }

    #[test]
    fn con_todo_apagado_notify_no_hace_nada() {
        let s = Settings { enabled: false, ..Settings::default() };
        assert!(!notify(s, Event::Timeout, "da igual"));
    }
}
