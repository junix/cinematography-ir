    use super::*;
    use crate::model::Scene;

    #[test]
    fn palette_wraps_with_tint() {
        let first = color_for_index(0);
        let wrapped = color_for_index(PALETTE.len());
        assert_eq!(first.name, wrapped.name);
        assert_ne!(first.rgb, wrapped.rgb);
        // One quarter of the way to white in the implementation's integer
        // channel math: 220 + (255-220)/4, 50 + (255-50)/4, 47 + (255-47)/4.
        assert_eq!(wrapped.rgb, Rgb { r: 228, g: 101, b: 99 });
    }

    #[test]
    fn tint_lightens_every_round_then_clamps_at_three() {
        let rounds: Vec<Rgb> = (0..5)
            .map(|round| color_for_index(round * PALETTE.len()).rgb)
            .collect();
        assert_eq!(
            rounds,
            vec![
                Rgb { r: 220, g: 50, b: 47 },
                Rgb { r: 228, g: 101, b: 99 },
                Rgb { r: 237, g: 152, b: 151 },
                Rgb { r: 246, g: 203, b: 203 },
                // round.min(3): a fifth wrap stays at three quarters to white.
                Rgb { r: 246, g: 203, b: 203 },
            ]
        );
    }

    #[test]
    fn scene_palette_assigns_colours_by_sorted_id_rank() {
        let mut scene: Scene = serde_yaml::from_str(
            r#"
id: s
title: S
duration_frames: 24
subjects:
  - { id: zulu, name: Z, kind: character }
  - { id: alpha, name: A, kind: character }
  - { id: mike, name: M, kind: character }
"#,
        )
        .unwrap();
        let palette = scene_palette(&scene);
        // Assignment follows the sorted-id rank, not declaration order.
        assert_eq!(palette["alpha"].name, "red");
        assert_eq!(palette["mike"].name, "blue");
        assert_eq!(palette["zulu"].name, "green");
        let ids: Vec<&str> = palette.keys().map(String::as_str).collect();
        assert_eq!(ids, vec!["alpha", "mike", "zulu"]);

        // Duplicate ids collapse to one entry without consuming a rank.
        scene.subjects.push(scene.subjects[1].clone());
        let palette = scene_palette(&scene);
        assert_eq!(palette.len(), 3);
        assert_eq!(palette["mike"].name, "blue");
    }

    #[test]
    fn hex_formatting() {
        assert_eq!(
            Rgb {
                r: 255,
                g: 0,
                b: 16
            }
            .hex(),
            "#ff0010"
        );
    }
