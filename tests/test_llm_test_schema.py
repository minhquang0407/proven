import pytest

from proven.core.llm_test_schema import LLMGeneratedTest, parse_generated_test, parse_repair_response, validate_generated_test


def test_validate_generated_test_accepts_safe_pytest_code():
    generated = LLMGeneratedTest(
        test_file='tests/test_example.py',
        test_names=['test_example_behavior'],
        code='def test_example_behavior():\n    assert 1 + 1 == 2\n',
        rationale='basic behavior',
    )

    assert validate_generated_test(generated) is True


def test_validate_generated_test_rejects_unsafe_path():
    generated = LLMGeneratedTest(
        test_file='../test_example.py',
        test_names=['test_example_behavior'],
        code='def test_example_behavior():\n    assert True\n',
        rationale='bad path',
    )

    with pytest.raises(ValueError, match='tests/'):
        validate_generated_test(generated)


def test_validate_generated_test_rejects_code_without_test_function():
    generated = LLMGeneratedTest(
        test_file='tests/test_example.py',
        test_names=['test_example_behavior'],
        code='def helper():\n    return True\n',
        rationale='no test',
    )

    with pytest.raises(ValueError, match='at least one pytest'):
        validate_generated_test(generated)


def test_validate_generated_test_rejects_unsafe_patterns():
    generated = LLMGeneratedTest(
        test_file='tests/test_example.py',
        test_names=['test_example_behavior'],
        code='import subprocess\n\ndef test_example_behavior():\n    assert True\n',
        rationale='unsafe',
    )

    with pytest.raises(ValueError, match='unsafe pattern'):
        validate_generated_test(generated)


def test_parse_generated_test_extracts_json_from_markdown_fence():
    text = '''```json
{"test_file":"tests/test_x.py","test_names":["test_x"],"code":"def test_x():\\n    assert True\\n","rationale":"ok"}
```'''

    generated = parse_generated_test(text)

    assert generated.test_file == 'tests/test_x.py'
    assert generated.test_names == ['test_x']


def test_parse_repair_response_validates_fixed_code():
    text = '''{"action":"replace_generated_block","code":"def test_fixed():\\n    assert True\\n","explanation":"fixed"}'''

    code, explanation = parse_repair_response(text)

    assert 'test_fixed' in code
    assert explanation == 'fixed'

