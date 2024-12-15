document.addEventListener('DOMContentLoaded', () => {
  // Select the form element
  const completeform = document.getElementById('completeform');

  if (completeform) {
    // Add event listener for keydown events
    document.addEventListener('keydown', (event) => {
      // Check if the pressed key is 'c'
      if (event.key === 'c' || event.key === 'C') {
        // Prevent the default behavior
        event.preventDefault();
        // Submit the form
        completeform.submit();
      }
    });
  }
  // Select the form element
  const skipform = document.getElementById('skipform');

  if (skipform) {
    // Add event listener for keydown events
    document.addEventListener('keydown', (event) => {
      // Check if the pressed key is 'c'
      if (event.key === 's' || event.key === 'S') {
        // Prevent the default behavior
        event.preventDefault();
        // Submit the form
        skipform.submit();
      }
    });
  }
});
