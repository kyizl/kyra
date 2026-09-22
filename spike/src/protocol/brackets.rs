#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerInfoBracket {
    LegacyAction,
    LegacyActionWithCrypto,
    BitflagsJsonName,
    BitflagsNbtName,
    BitflagsNbtNameListOrder,
    BitflagsNbtNameListOrderHat,
    Latest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamsBracket {
    Legacy8,
    Legacy9To12,
    ModernString,
    ModernNbt,
    ModernNbtMappedEnums,
    NestedContainer,
}

pub fn player_info_bracket(protocol_version: i32) -> Option<PlayerInfoBracket> {
    use PlayerInfoBracket::*;
    match protocol_version {
        i32::MIN..=758 => Some(LegacyAction),
        759..=760 => Some(LegacyActionWithCrypto),
        761..=764 => Some(BitflagsJsonName),
        765..=767 => Some(BitflagsNbtName),
        768 => Some(BitflagsNbtNameListOrder),
        769..=772 => Some(BitflagsNbtNameListOrderHat),
        773..=775 => Some(Latest),
        _ => None,
    }
}

pub fn teams_bracket(protocol_version: i32) -> Option<TeamsBracket> {
    use TeamsBracket::*;
    match protocol_version {
        47 => Some(Legacy8),
        48..=340 => Some(Legacy9To12),
        393..=764 => Some(ModernString),
        765..=769 => Some(ModernNbt),
        770 => Some(ModernNbtMappedEnums),
        771..=775 => Some(NestedContainer),
        _ => None,
    }
}

pub fn teams_packet_name(protocol_version: i32) -> &'static str {
    if protocol_version == 47 {
        "scoreboard_team"
    } else {
        "teams"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_info_bracket_boundaries_are_exact() {
        assert_eq!(
            player_info_bracket(47),
            Some(PlayerInfoBracket::LegacyAction)
        );
        assert_eq!(
            player_info_bracket(758),
            Some(PlayerInfoBracket::LegacyAction)
        );
        assert_eq!(
            player_info_bracket(759),
            Some(PlayerInfoBracket::LegacyActionWithCrypto)
        );
        assert_eq!(
            player_info_bracket(760),
            Some(PlayerInfoBracket::LegacyActionWithCrypto)
        );
        assert_eq!(
            player_info_bracket(761),
            Some(PlayerInfoBracket::BitflagsJsonName)
        );
        assert_eq!(
            player_info_bracket(764),
            Some(PlayerInfoBracket::BitflagsJsonName)
        );
        assert_eq!(
            player_info_bracket(765),
            Some(PlayerInfoBracket::BitflagsNbtName)
        );
        assert_eq!(
            player_info_bracket(767),
            Some(PlayerInfoBracket::BitflagsNbtName)
        );
        assert_eq!(
            player_info_bracket(768),
            Some(PlayerInfoBracket::BitflagsNbtNameListOrder)
        );
        assert_eq!(
            player_info_bracket(769),
            Some(PlayerInfoBracket::BitflagsNbtNameListOrderHat)
        );
        assert_eq!(
            player_info_bracket(772),
            Some(PlayerInfoBracket::BitflagsNbtNameListOrderHat)
        );
        assert_eq!(player_info_bracket(773), Some(PlayerInfoBracket::Latest));
        assert_eq!(player_info_bracket(775), Some(PlayerInfoBracket::Latest));
        assert_eq!(player_info_bracket(776), None);
    }

    #[test]
    fn teams_bracket_boundaries_are_exact() {
        assert_eq!(teams_bracket(47), Some(TeamsBracket::Legacy8));
        assert_eq!(teams_bracket(107), Some(TeamsBracket::Legacy9To12));
        assert_eq!(teams_bracket(340), Some(TeamsBracket::Legacy9To12));
        assert_eq!(teams_bracket(393), Some(TeamsBracket::ModernString));
        assert_eq!(teams_bracket(764), Some(TeamsBracket::ModernString));
        assert_eq!(teams_bracket(765), Some(TeamsBracket::ModernNbt));
        assert_eq!(teams_bracket(769), Some(TeamsBracket::ModernNbt));
        assert_eq!(teams_bracket(770), Some(TeamsBracket::ModernNbtMappedEnums));
        assert_eq!(teams_bracket(771), Some(TeamsBracket::NestedContainer));
        assert_eq!(teams_bracket(775), Some(TeamsBracket::NestedContainer));
        assert_eq!(teams_bracket(776), None);
        assert_eq!(teams_bracket(46), None);
        assert_eq!(teams_bracket(341), None);
        assert_eq!(teams_bracket(392), None);
    }

    #[test]
    fn teams_packet_name_matches_1_8_exception() {
        assert_eq!(teams_packet_name(47), "scoreboard_team");
        assert_eq!(teams_packet_name(107), "teams");
        assert_eq!(teams_packet_name(775), "teams");
    }
}
