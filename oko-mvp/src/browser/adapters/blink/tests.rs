    use super::*;
use crate::browser::ports::engine::BrowserEnginePort;

    #[test]
    fn blink_identity_without_link() {
        let engine = BlinkBrowserEngineAdapter::new();
        assert_eq!(engine.engine_name(), "blink");
        assert!(!engine.is_experimental());
        assert!(engine.capabilities().supports_js);
    }

    #[test]
    fn blink_unlinked_reports_unavailable() {
        let engine = BlinkBrowserEngineAdapter::new();
        assert!(engine.initialize().is_err());
        assert!(engine
            .create_session(crate::browser::ports::engine::BrowserConfig {
                profile: "default".into(),
                url: None,
                off_the_record: false,
            })
            .is_err());
    }

    #[test]
    fn blink_crash_is_isolated() {
        let engine = BlinkBrowserEngineAdapter::new();
        engine.simulate_crash("renderer process terminated");
        assert!(engine.initialize().is_err());
        assert_eq!(
            engine.last_crash_reason(),
            Some("renderer process terminated".into())
        );
        engine.clear_crash();
        // Still unavailable without a linked embedder, but crash state cleared.
        assert_eq!(engine.last_crash_reason(), None);
    }

    #[test]
    fn one_engine_per_instance() {
        let a = BlinkBrowserEngineAdapter::new();
        let b = BlinkBrowserEngineAdapter::new();
        assert_eq!(a.engine_name(), "blink");
        assert_eq!(b.engine_name(), "blink");
        // No shared internal state between instances (SRD §2.2).
    }
