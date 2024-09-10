document.getElementById("vanityForm").addEventListener("submit", function(event) {
    event.preventDefault();

    const prefix = document.getElementById("prefix").value;
    const suffix = document.getElementById("suffix").value;
    const keyLength = parseInt(document.getElementById("keyLength").value);
    const addressType = document.getElementById("addressType").value;

    // Prefix ve Suffix doğrulaması
    if (prefix === "" || suffix === "") {
        alert("Prefix ve Suffix boş olamaz.");
        return;
    }

    // Geçersiz karakter kontrolü (yalnızca alfasayısal karakterlere izin ver)
    const regex = /^[a-zA-Z0-9]+$/;
    if (!regex.test(prefix) || !regex.test(suffix)) {
        alert("Prefix ve Suffix yalnızca alfasayısal karakterler içerebilir.");
        return;
    }

    fetch("/api/generate", {
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
    })
    .then(response => {
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return response.json();
    })
    .then(data => {
        if (data.address) {
            document.getElementById("result").innerHTML = `
                <p><strong>Address:</strong> ${data.address} <button onclick="copyToClipboard('${data.address}')">Kopyala</button></p>
                <p><strong>Private Key (WIF):</strong> ${data.private_key} <button onclick="copyToClipboard('${data.private_key}')">Kopyala</button></p>
                <p><strong>Public Key:</strong> ${data.public_key}</p>
                <p><strong>Address Type:</strong> ${data.address_type}</p>
                <p><strong>Address Hash:</strong> ${data.address_hash}</p>
            `;
        } else {
            document.getElementById("result").innerHTML = "<p>No address found. Try again!</p>";
        }
    })
    .catch(error => {
        document.getElementById("result").innerHTML = `<p>An error occurred: ${error.message}</p>`;
        console.error("Error:", error);
    });
});

// Kopyalama fonksiyonu
function copyToClipboard(text) {
    const tempInput = document.createElement("input");
    tempInput.value = text;
    document.body.appendChild(tempInput);
    tempInput.select();
    document.execCommand("copy");
    document.body.removeChild(tempInput);
    alert("Kopyalandı: " + text);
}
