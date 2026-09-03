export const charge = () =>
  fetch("http://payments-api:3000/charge", { method: "POST" });
