import pytest

from proven.core.llm_provider import GeminiProvider, LLMConfig, TemplateFallbackProvider, build_llm_provider, load_llm_config


def test_load_llm_config_defaults_to_template(monkeypatch):
    for key in [
        'SOFTGNN_LLM_PROVIDER',
        'SOFTGNN_LLM_BASE_URL',
        'SOFTGNN_LLM_MODEL',
        'SOFTGNN_LLM_API_KEY',
        'SOFTGNN_LLM_TIMEOUT',
    ]:
        monkeypatch.delenv(key, raising=False)

    config = load_llm_config()

    assert config.provider == 'template'
    assert config.timeout == 60.0


def test_build_template_fallback_provider():
    provider = build_llm_provider(LLMConfig(provider='template'))

    assert isinstance(provider, TemplateFallbackProvider)
    assert provider.available is False


def test_openai_compatible_requires_base_url_and_model():
    with pytest.raises(ValueError):
        build_llm_provider(LLMConfig(provider='openai-compatible', model='model-only'))
    with pytest.raises(ValueError):
        build_llm_provider(LLMConfig(provider='openai-compatible', base_url='http://localhost:11434/v1'))


def test_gemini_provider_requires_model_and_api_key():
    with pytest.raises(ValueError):
        build_llm_provider(LLMConfig(provider='gemini', api_key='key-only'))
    with pytest.raises(ValueError):
        build_llm_provider(LLMConfig(provider='gemini', model='gemini-3-flash'))


def test_build_gemini_provider():
    provider = build_llm_provider(LLMConfig(provider='gemini', model='gemini-3-flash', api_key='secret'))

    assert isinstance(provider, GeminiProvider)
    assert provider.available is True

