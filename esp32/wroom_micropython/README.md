# ESP32 Spin Coater Controller

This project runs on an ESP32 using MicroPython. It provides a touchscreen interface to control a brushless DC motor via an ESC, utilizing a PID loop and Hall effect sensor for precise RPM control.

## 🛠 Prerequisites

1. **ESP32 Development Board** (Flashed with the latest MicroPython firmware).
2. **Data Cable** (Micro-USB or USB-C, ensuring it handles data, not just charging).
3. **Python 3** installed on your computer.
4. **Code Editor:** Any IDE or text editor you prefer (e.g., Visual Studio Code).
5. **mpremote:** The official MicroPython command-line tool.

---

## 📁 Required Files

Ensure you have the following files in your project folder:

* `main.py` - The core application logic (GUI, PID loop, hardware control).
* `ili9341.py` - The driver for the TFT display.
* `xpt2046.py` - The driver for the touch controller.

---

## 🚀 Terminal / CLI Upload Guide

Using `mpremote` allows you to manage your files directly from your terminal (like the integrated terminal in VS Code) without needing a dedicated MicroPython IDE.

### Step 1: Install mpremote
Open your terminal and install the tool via pip:
```bash
pip install mpremote
```

### Step 2: Connect your ESP32
Plug your ESP32 into your computer. You can verify `mpremote` sees your board by running:
```bash
mpremote devs
```
*(This will list the serial port your ESP32 is connected to.)*

### Step 3: Upload the Files
In your terminal, navigate to the folder containing your code:
```bash
cd path/to/your/project/folder
```
Then, copy the required files to the root directory (`:`) of the ESP32:
```bash
mpremote cp main.py ili9341.py xpt2046.py :
```
*(Note: During development, if you only update one file, you can just run `mpremote cp main.py :` to overwrite it quickly).*

### Step 4: Reboot and Run
Because the main script is named `main.py`, MicroPython will automatically execute it every time the board powers on. 

To start it immediately, you can simply press the physical **Reset/EN** button on your ESP32 board. 

**Debugging Tip:** If you want to see the live terminal output (like `print()` statements or errors), connect to the REPL and trigger a soft-reset:
```bash
mpremote repl
```
*(Once in the REPL, press `Ctrl + D` on your keyboard to trigger a soft reboot and watch the code run).*

---

## 🔌 Hardware Pinout Reference

For quick reference, here is the wiring configuration expected by the code:

### Display (ILI9341 - SPI 2)
* **SCK (Clock):** GPIO 18
* **MOSI:** GPIO 23
* **MISO:** GPIO 19
* **DC (Data/Command):** GPIO 2
* **CS (Chip Select):** GPIO 15
* **RST (Reset):** GPIO 4

### Touch Screen (XPT2046 - SPI 1)
* **SCK:** GPIO 14
* **MOSI:** GPIO 13
* **MISO:** GPIO 12
* **CS (Chip Select):** GPIO 33
* **IRQ (Interrupt):** GPIO 25

### Motor & Sensors
* **ESC PWM Signal:** GPIO 26
* **Hall Effect Sensor:** GPIO 27 (Requires 10k pull-up resistor if not built into the sensor module)