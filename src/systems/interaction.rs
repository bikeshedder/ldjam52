use bevy::prelude::*;

use crate::components::{interaction::Interaction, player::Player};

/// Marker for the text entity displaying the available interactions.
#[derive(Component)]
pub struct InteractionText;

pub fn detect_interaction(
    player: Single<(&Player, &Transform)>,
    interaction_query: Query<&Interaction>,
    mut text: Single<&mut Text, With<InteractionText>>,
) {
    let (player, player_transform) = *player;
    let names = interaction_query
        .iter()
        .filter(|interaction| {
            ((player_transform.translation + player.center).distance(interaction.center))
                <= f32::from(interaction.max_distance)
        })
        .map(|interaction| interaction.name.as_str())
        .collect::<Vec<_>>();
    text.0 = if names.is_empty() {
        String::from("No interactions available")
    } else {
        format!("Interactions: {}", names.join(" "))
    };
}
