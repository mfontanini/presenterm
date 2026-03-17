document.addEventListener('DOMContentLoaded', function() {
  const allLines = document.querySelectorAll('body > div');
  const pageBreakMarkers = document.querySelectorAll('.container');

  function getCurrentPageIndex() {
    const hash = window.location.hash;
    const match = hash.match(/^#slide-(\d+)$/);
    if (match) {
      const idx = parseInt(match[1], 10);
      const max = pageBreakMarkers.length;
      if (idx >= 0 && idx < max) return idx;
    }
    return 0;
  }

  let currentPageIndex = getCurrentPageIndex();

  function showCurrentPage() {
    allLines.forEach((line) => {
      line.classList.add('hidden');
    });

    allLines[currentPageIndex].classList.remove('hidden');
    history.replaceState(null, '', '#slide-' + currentPageIndex);
  }

  function scaler() {
    var w = document.documentElement.clientWidth;
    var h = document.documentElement.clientHeight;
    let widthScaledAmount= w/originalWidth;
    let heightScaledAmount= h/originalHeight;
    let scaledAmount = Math.min(widthScaledAmount, heightScaledAmount);
    document.querySelector("body").style.transform = `scale(${scaledAmount})`;
  }

  function handleKeyPress(event) {
    if (event.key === 'ArrowLeft') {
      if (currentPageIndex > 0) {
        currentPageIndex--;
        showCurrentPage();
      }
    } else if (event.key === 'ArrowRight') {
      if (currentPageIndex < pageBreakMarkers.length - 1) {
        currentPageIndex++;
        showCurrentPage();
      }
    }
  }

  document.addEventListener('keydown', handleKeyPress);
  window.addEventListener("resize", scaler);

  scaler();
  showCurrentPage();
});

