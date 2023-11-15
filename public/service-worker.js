// This will listen for the 'install' event, which occurs when the service worker is first installed.
self.addEventListener('install', event => {
  console.log('Service Worker installing.');
});

// This listens for the 'activate' event, which occurs when the service worker is activated and ready to control pages.
self.addEventListener('activate', event => {
  console.log('Service Worker activated.');
});

// Listen for messages sent from the main application script.
self.addEventListener("message", (event) => {
  self.registration.showNotification("Hello World", {
    body: 'This is your minute notification!',
  });
});