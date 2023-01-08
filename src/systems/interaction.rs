use bevy::{
    prelude::{Query, Transform},
    text::Text,
};

use crate::components::{interaction::Interaction, player::Player};

pub fn detect_interaction(
    player_query: Query<(&Player, &Transform)>,
    interaction_query: Query<&Interaction>,
    mut text_query: Query<&mut Text>,
) {
    let (player, player_transform) = player_query.single();
    let interactions = interaction_query
        .iter()
        .filter(|interaction| {
            ((player_transform.translation + player.center).distance(interaction.center))
                <= f32::from(interaction.max_distance)
        })
        .collect::<Vec<_>>();
    let text = if interactions.is_empty() {
        String::from("No interactions available")
    } else {
        let names = interactions
            .iter()
            .map(|iact| iact.name.as_ref())
            .collect::<Vec<_>>();
        let names = names.join(" ");
        format!("Interactions: {}", names)
    };
    text_query.single_mut().sections[0].value = text;
}
