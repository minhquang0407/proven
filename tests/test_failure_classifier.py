from softgnn_advisor.core.failure_classifier import classify_pytest_failure, replanable_failures
from softgnn_advisor.core.test_generation_agent import PlanVerificationResult


def test_classify_environment_failure_skips_replan():
    output = 'import torch._dynamo\nimport sympy\nValueError: negative shift count'
    category, reason = classify_pytest_failure(output, source_file='src/train.py', test_file='tests/test_train.py')
    assert category == 'environment'
    assert 'replan' in reason


def test_classify_source_runtime_failure_skips_replan():
    output = 'src/train.py:54: in train_model\nE RuntimeError: mat1 and mat2 shapes cannot be multiplied'
    category, reason = classify_pytest_failure(output, source_file='src/train.py')
    assert category == 'source_or_complex_runtime'
    assert 'src/train.py' in reason


def test_replanable_failures_blocks_environment_result():
    result = PlanVerificationResult(
        target_id='FUNC:train_model',
        test_file='tests/test_train.py',
        pytest_target='tests/test_train.py::test_train_model_execution',
        returncode=1,
        output='torch._dynamo\nValueError: negative shift count',
        status='rolled_back',
    )
    allowed, blocked = replanable_failures([result], {'FUNC:train_model': 'src/train.py'})
    assert allowed == {}
    assert blocked and blocked[0][0] == 'FUNC:train_model'
