document.addEventListener('DOMContentLoaded', function() {
  const allLines = document.querySelectorAll('body > div');
  const pageBreakMarkers = document.querySelectorAll('.force-page-break');
  let currentPageIndex = 0;

  function showCurrentPage() {
    const startIndexOfCurrentPage = Array.from(allLines).indexOf(pageBreakMarkers[currentPageIndex]);
    let endIndexOfCurrentPage = allLines.length; 

    if (currentPageIndex < pageBreakMarkers.length - 1) {
      endIndexOfCurrentPage = Array.from(allLines).indexOf(pageBreakMarkers[currentPageIndex + 1]);
    }

    allLines.forEach((line, index) => {
      if (startIndexOfCurrentPage <= index && index < endIndexOfCurrentPage) {
        line.classList.remove('hidden'); 
      } else {
        line.classList.add('hidden');
      }
    });
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

  showCurrentPage();
});

