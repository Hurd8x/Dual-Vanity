document.getElementById("vanityForm").addEventListener("submit", async function(event) {
    event.preventDefault();

    const prefix = document.getElementById("prefix").value.trim();
    const suffix = document.getElementById("suffix").value.trim();
    const keyLength = parseInt(document.getElementById("keyLength").value);
    const addressType = document.getElementById("addressType").value;
    const resultContainer = document.getElementById("result");
    const loadingMessage = document.getElementById("loading");

    // Input validation
    if (prefix === "" || suffix === "") {
        alert("Prefix ve Suffix boş olamaz.");
        return;
    }

    const regex = /^[a-zA-Z0-9]+$/;
    if (!regex.test(prefix) || !regex.test(suffix)) {
        alert("Prefix ve Suffix yalnızca alfasayısal karakterler (a-z, A-Z, 0-9) içerebilir.");
        return;
    }

    // Show loading message
    loadingMessage.style.display = "block";
    resultContainer.innerHTML = "";

    try {
        const response = await fetch("/api/generate", {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            },
            body: JSON.stringify({
                prefix: prefix,
                suffix: suffix,
                key_length: keyLength,
                address_type: addressType
            })
        });

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        const data = await response.json();
        loadingMessage.style.display = "none";

        if (data.address) {
            resultContainer.innerHTML = `
                <p><strong>Address:</strong> ${data.address} 
                <button onclick="copyToClipboard('${data.address}')">Kopyala</button></p>
                <p><strong>Private Key (WIF):</strong> ${data.private_key} 
                <button onclick="copyToClipboard('${data.private_key}')">Kopyala</button></p>
                <p><strong>Public Key:</strong> ${data.public_key}</p>
                <p><strong>Address Type:</strong> ${data.address_type}</p>
            `;
        } else {
            resultContainer.innerHTML = "<p>No address found. Try again!</p>";
        }
    } catch (error) {
        loadingMessage.style.display = "none";
        resultContainer.innerHTML = `<p>An error occurred: ${error.message}</p>`;
        console.error("Error:", error);
    }
});

// Modern clipboard API
function copyToClipboard(text) {
    navigator.clipboard.writeText(text).then(() => {
        alert("Kopyalandı: " + text);
    }).catch(err => {
        console.error("Kopyalama hatası: ", err);
    });
}
