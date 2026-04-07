document.addEventListener('DOMContentLoaded', function() {
  const slides = document.querySelectorAll('body > div');
  const totalSlides = document.querySelectorAll('.container').length;

  function getInitialSlideIndex() {
    const hash = window.location.hash;
    const match = hash.match(/^#slide-(\d+)$/);
    if (match) {
      const idx = parseInt(match[1], 10);
      if (idx >= 0 && idx < totalSlides) return idx;
    }
    return 0;
  }

  let currentSlideIndex = getInitialSlideIndex();

  function showCurrentSlide() {
    slides.forEach((slide) => {
      slide.classList.add('hidden');
    });
    slides[currentSlideIndex].classList.remove('hidden');
    window.location.hash = 'slide-' + currentSlideIndex;
  }

  function showNextSlide() {
    if (currentSlideIndex >= totalSlides - 1) return;
    currentSlideIndex++;
    showCurrentSlide();
  }

  function showPreviousSlide() {
    if (currentSlideIndex <= 0) return;
    currentSlideIndex--;
    showCurrentSlide();
  }

  function scaleToFit() {
    const w = window.innerWidth;
    const h = window.innerHeight;
    const scale = Math.min(w / originalWidth, h / originalHeight);
    const offsetX = (w - originalWidth * scale) / 2;
    const offsetY = (h - originalHeight * scale) / 2;
    const body = document.querySelector("body");
    body.style.transform = `translate(${offsetX}px, ${offsetY}px) scale(${scale})`;
  }

  function handleKeyPress(event) {
    switch (event.key) {
      case 'ArrowLeft':
        showPreviousSlide();
        break;
      case 'ArrowRight':
        showNextSlide();
        break;
    }
  }

  let touchStartX = 0;
  let touchStartY = 0;
  const swipeThreshold = 50;

  function handleTouchStart(event) {
    touchStartX = event.touches[0].clientX;
    touchStartY = event.touches[0].clientY;
  }

  function handleTouchEnd(event) {
    const dx = event.changedTouches[0].clientX - touchStartX;
    const dy = event.changedTouches[0].clientY - touchStartY;
    if (Math.abs(dx) < swipeThreshold || Math.abs(dy) > Math.abs(dx)) return;
    const swipedLeft = dx < 0;
    if (swipedLeft) {
      showNextSlide();
      return;
    }
    showPreviousSlide();
  }

  function handleClick(event) {
    if (event.clientX < document.documentElement.clientWidth / 3) {
      showPreviousSlide();
      return;
    }
    showNextSlide();
  }

  document.addEventListener('keydown', handleKeyPress);
  document.addEventListener('touchstart', handleTouchStart, { passive: true });
  document.addEventListener('touchend', handleTouchEnd);
  document.addEventListener('click', handleClick);
  window.addEventListener("resize", scaleToFit);

  scaleToFit();
  showCurrentSlide();
});
