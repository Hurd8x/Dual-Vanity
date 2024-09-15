import os
import hashlib
import binascii
import base58
import secp256k1
import random
import asyncio
from multiprocessing import Manager, Pool

# Compressed public key üretme (libsecp256k1 kullanarak)
def private_key_to_compressed_public_key(private_key):
    privkey = secp256k1.PrivateKey(bytes.fromhex(private_key))
    return privkey.pubkey.serialize(compressed=True).hex()

# RIPEMD-160 hash hesaplama
def public_key_to_ripemd160(public_key):
    sha256_hash = hashlib.sha256(binascii.unhexlify(public_key)).hexdigest()
    ripemd160 = hashlib.new('ripemd160')
    ripemd160.update(binascii.unhexlify(sha256_hash))
    return ripemd160.hexdigest()

# Bitcoin adresi oluşturma
def public_key_to_p2pkh_address(public_key):
    ripemd160_hash = public_key_to_ripemd160(public_key)
    network_byte = '00'
    hashed_public_key_with_network_byte = network_byte + ripemd160_hash
    sha256_hash2 = hashlib.sha256(binascii.unhexlify(hashed_public_key_with_network_byte)).hexdigest()
    sha256_hash3 = hashlib.sha256(binascii.unhexlify(sha256_hash2)).hexdigest()
    checksum = sha256_hash3[:8]
    final_address_hex = hashed_public_key_with_network_byte + checksum
    return base58.b58encode(binascii.unhexlify(final_address_hex)).decode('utf-8'), ripemd160_hash

# Anahtarın geçerliliğini kontrol etme
def check_key(private_key, prefix, suffix, file_lock):
    try:
        private_key_hex = hex(private_key)[2:].zfill(64)
        compressed_public_key = private_key_to_compressed_public_key(private_key_hex)
        address, ripemd160_hash = public_key_to_p2pkh_address(compressed_public_key)
        
        if address.startswith(prefix) and address.endswith(suffix):
            with file_lock:  # Dosya yazma işlemi sırasında kilitleme
                with open("keyfound.txt", "a") as f:
                    f.write(f"Match found!\nPrivate Key: {private_key_hex}\nCompressed Public Key: {compressed_public_key}\nRIPEMD-160: {ripemd160_hash}\nAddress: {address}\n\n")
            print(f"Match found!\nPrivate Key: {private_key_hex}\nCompressed Public Key: {compressed_public_key}\nRIPEMD-160: {ripemd160_hash}\nAddress: {address}")
            return True
    except Exception as e:
        print(f"Hata: {e}")
    return False

# Python random modülü ile batch private key üretme (start_range ve end_range arasında)
def generate_random_private_keys(batch_size, start_range, end_range):
    # Büyük tamsayı aralıkları için random.randint kullanılıyor
    return [random.randint(start_range, end_range) for _ in range(batch_size)]

# Async anahtar tarama fonksiyonu (Python random ile batch taraması eklenmiş)
async def scan_random_keys_in_range_async(start, end, prefix, suffix, file_lock, batch_size=1000):
    total_keys_checked = 0
    while True:
        # Python random kullanarak batch halinde private key üretimi
        private_keys = generate_random_private_keys(batch_size, start, end)
        
        # Her batch'deki key'leri tarama
        for private_key in private_keys:
            if check_key(private_key, prefix, suffix, file_lock):
                pass
            total_keys_checked += 1
    

# Paralel tarama için async yöntem (multiprocessing ile dinamik görev dağılımı)
async def parallel_key_scanning_with_async(start, end, prefix, suffix, file_lock, num_tasks=8, batch_size=1000):
    tasks = []
    for _ in range(num_tasks):
        tasks.append(scan_random_keys_in_range_async(start, end, prefix, suffix, file_lock, batch_size))
    await asyncio.gather(*tasks)

# Dinamik görev dağılımı (multiprocessing ile Queue kullanımı)
def scan_keys_with_dynamic_range(task_queue, prefix, suffix, file_lock):
    while not task_queue.empty():
        start, end = task_queue.get()
        asyncio.run(parallel_key_scanning_with_async(start, end, prefix, suffix, file_lock, num_tasks=4))

def parallel_key_scanning_with_dynamic_range(start, end, prefix, suffix, num_processes=8, batch_size=10000):
    range_size = (end - start) // num_processes
    manager = Manager()
    task_queue = manager.Queue()
    file_lock = manager.Lock()  # Manager üzerinden kilit oluşturuluyor

    # İşleri kuyruğa ekleyelim
    for i in range(num_processes):
        task_queue.put((start + i * range_size, start + (i + 1) * range_size))

    with Pool(processes=num_processes) as pool:
        pool.starmap(scan_keys_with_dynamic_range, [(task_queue, prefix, suffix, file_lock) for _ in range(num_processes)])

# Anahtar aralığı
start_range = 0x40000000000000000  # Taramanın başlayacağı aralık
end_range = 0x7ffffffffffffffff    # Taramanın sona ereceği aralık

# Aranacak prefix ve suffix
prefix = "1B"
suffix = "W9"

# Taramayı başlatma
parallel_key_scanning_with_dynamic_range(start_range, end_range, prefix, suffix, num_processes=8, batch_size=1000)
