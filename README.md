# Buddy GPS Tracking

![Buddy GPS Tracking](assets/home.png)

A web application to track and display GPS data from an ESP32 device, designed for tracking a pet.

## Features

*   **Real-time Data Display:** View the latest GPS data from your device in a clean, sortable table.
*   **Data Refresh:** Manually refresh the data to get the latest updates.
*   **CSV Export:** Download all stored GPS data as a CSV file for further analysis.

## Tech Stack

*   **Frontend:** [Leptos](https://leptos.dev/) (Rust)
*   **Backend:** [Actix](https://actix.rs/) (Rust)
*   **Styling:** [Tailwind CSS](https://tailwindcss.com/)
*   **Testing:** [Playwright](https://playwright.dev/)

## Getting Started

### Prerequisites

*   [Rust](https://www.rust-lang.org/tools/install)
*   [cargo-leptos](https://github.com/leptos-rs/leptos/tree/main/cargo-leptos)
*   [Node.js](https://nodejs.org/en/) (for end-to-end testing)

### Installation and Running

1.  **Install `cargo-leptos`:**
    ```bash
    cargo install cargo-leptos
    ```

2.  **Build and run the application:**
    ```bash
    cargo leptos watch
    ```

3.  **Open your browser** to `http://127.0.0.1:3000`.

## Running Tests

1.  **Install Node.js dependencies:**
    ```bash
    npm install --prefix end2end
    ```

2.  **Run the end-to-end tests:**
    ```bash
    cargo leptos end2end
    ```
