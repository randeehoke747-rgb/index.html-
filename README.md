
# Donation Bot

This is a simple Flask donation bot with links to Cash App, Venmo, and Bitcoin.

## 🚀 How to Run Locally

1. Install dependencies:
   ```bash
   pip install -r requirements.txt
   ```

2. Run the app:
   ```bash
   python app.py
   ```

3. Open in your browser:
   ```
   http://127.0.0.1:5000
   ```

---

## 🌍 How to Deploy (Render Example)

1. Create a free account at [Render](https://render.com).
2. Create a **New Web Service**.
3. Connect your GitHub repo (or upload this code).
4. Set **Build Command**:
   ```bash
   pip install -r requirements.txt
   ```
5. Set **Start Command**:
   ```bash
   gunicorn app:app
   ```
6. Deploy → Render gives you a live URL.

---

## 🌐 Connect a Custom Domain

1. Buy a domain (Namecheap, Google Domains, etc.).
2. Go to your domain's DNS settings.
3. Add a **CNAME record** pointing to your Render URL.
4. In Render settings, add your domain under **Custom Domains**.
5. Enable HTTPS (Render gives free SSL).

---

## 💰 Donation Links Included

- **Cash App** → [https://cash.app/$wellfindit1taxing][https://cash.app/$wellfindit1taxing]
- **Venmo** → [https://venmo.com/@randee74](https://venmo.com/@randee74)
- **Bitcoin** → [3ETpuWUdWJhfLy5h6PgviHo4R3GqJqqn5g]

You can edit `templates/index.html` to change these links or add more.
