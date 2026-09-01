from tasks.mirror.select_theme_pack import _theme_alias_matches


def test_numbered_theme_alias_does_not_match_a_longer_line_number() -> None:
    assert _theme_alias_matches("Line 1", "Line 1")
    assert not _theme_alias_matches("Line 10", "Line 1")
    assert _theme_alias_matches("Line 10", "Line 10")
