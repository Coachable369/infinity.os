(() => {
    const element = document.activeElement;
    if (element && element.type === 'password') return '';
    const text = element && typeof element.selectionStart === 'number'
        ? element.value.substring(element.selectionStart, element.selectionEnd)
        : String(getSelection());
    return text && text.length <= 512
        ? 'https://www.google.com/search?q=' + encodeURIComponent(text) : '';
})()
