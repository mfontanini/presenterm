document.addEventListener('DOMContentLoaded', function() {
  const allLines = document.querySelectorAll('body > div');
  const pageBreakMarkers = document.querySelectorAll('.container');
  let currentPageIndex = 0;


  function showCurrentPage() {
    allLines.forEach((line) => {
      line.classList.add('hidden');
    });

    allLines[currentPageIndex].classList.remove('hidden');
  }


  function scaler() {
    var w = document.documentElement.clientWidth;
    let scaledAmount= w/originalWidth;
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

