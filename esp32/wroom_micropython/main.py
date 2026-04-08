"""
================================================================================
Spin Coater ESC & PID Controller
================================================================================
Description: 
This script controls a brushless DC motor via an ESC for a spin coater application.
It features a touchscreen UI (ILI9341 + XPT2046) to set target RPM and duration.
It uses a Hall effect sensor to measure actual RPM and implements a PID control 
loop with non-linear feedforward (via a Lookup Table) to maintain accurate speeds.

Key Features:
- PID control with Anti-Windup.
- Non-linear feedforward mapping (Lookup Table).
- "Kick-Start" to overcome static friction / cogging.
- "Spool-up Interlock" to prevent integral windup during ESC startup delay.
- Auto-Calibration routine to dynamically map the motor's RPM-to-PWM curve.

Hardware Setup:
- Display: ILI9341 (SPI 2)
- Touch: XPT2046 (SPI 1)
- Motor PWM Pin: GPIO 26
- Hall Sensor Pin: GPIO 27

TODO:
 - In the future remap pins 12-15 for debugger chip
================================================================================
"""

import math
import time
import machine
from machine import Pin, SPI
import ili9341
from xpt2046 import Touch

# ==============================================================================
# 1. HARDWARE & PERIPHERAL SETUP
# ==============================================================================
machine.freq(240000000) # Set CPU to 240MHz for faster math & UI updates

# --- Display SPI ---
spi_display = SPI(2, baudrate=40000000, sck=Pin(18), mosi=Pin(23), miso=Pin(19))
display = ili9341.Display(spi_display, dc=Pin(2), cs=Pin(15), rst=Pin(4))

# --- Touch SPI ---
spi_touch = SPI(1, baudrate=2000000, sck=Pin(14), mosi=Pin(13), miso=Pin(12))
touch = Touch(spi_touch, cs=Pin(33))
touch_irq = Pin(25, Pin.IN, Pin.PULL_UP)

# --- Motor ESC (PWM) ---
motor_pin = Pin(26, Pin.OUT)
motor_pwm = machine.PWM(motor_pin, freq=50) # 50Hz is standard for RC ESCs
NEUTRAL_PWM = 1500

def set_esc_pwm(width_us):
    """Safely limits the PWM signal and applies it to the motor."""
    safe_width = max(1000, min(2000, int(width_us)))
    motor_pwm.duty_ns(safe_width * 1000)

set_esc_pwm(NEUTRAL_PWM) # Arm ESC / Ensure it boots in neutral

# --- Hall Sensor (Interrupt) ---
hall_pin = Pin(27, Pin.IN, Pin.PULL_UP)
pulse_count = 0

def hall_isr(pin):
    """Interrupt Service Routine to count hall sensor pulses."""
    global pulse_count
    pulse_count += 1

# Trigger on falling edge (magnet passing sensor)
hall_pin.irq(trigger=Pin.IRQ_FALLING, handler=hall_isr)


# ==============================================================================
# 2. DYNAMIC DRIVETRAIN & ESC VARIABLES
# ==============================================================================
GEAR_MOTOR_TEETH = 20.0
GEAR_CARRIER_TEETH = 74.0
GEAR_RATIO = GEAR_CARRIER_TEETH / GEAR_MOTOR_TEETH

# Overcome cogging/static friction of sensorless brushless motors
KICKSTART_PWM = 1580
KICKSTART_MS = 350

# Default (3300KV@12V) Non-Linear curve (Overwritten by Auto-Calibrate)
# Format: (Motor_RPM, PWM_us). MUST be in strictly ascending order by RPM.
THROTTLE_CURVE = [
    (0,      1500), 
    (9800,   1580), 
    (17900,  1600),
    (24900,  1625),
    (29500,  1650),
    (32500,  1675),
    (34500,  1700),
    (38100,  1800),
    (38500,  1900),
    (38500,  2000)
]

def estimate_base_pwm(target_motor_rpm):
    """Calculates base PWM feedforward via linear interpolation of THROTTLE_CURVE."""
    # Clamp to boundaries
    if target_motor_rpm <= THROTTLE_CURVE[0][0]: return THROTTLE_CURVE[0][1]
    if target_motor_rpm >= THROTTLE_CURVE[-1][0]: return THROTTLE_CURVE[-1][1]
        
    # Interpolate between the nearest two mapped points
    for i in range(len(THROTTLE_CURVE) - 1):
        rpm_low, pwm_low = THROTTLE_CURVE[i]
        rpm_high, pwm_high = THROTTLE_CURVE[i+1]
        
        if rpm_low <= target_motor_rpm <= rpm_high:
            if rpm_high == rpm_low: return pwm_low # Prevent divide by zero
            ratio = (target_motor_rpm - rpm_low) / (rpm_high - rpm_low)
            return pwm_low + (ratio * (pwm_high - pwm_low))
            
    return 1500 # Safe fallback


# ==============================================================================
# 3. PID & STATE VARIABLES
# ==============================================================================
target_carrier_rpm = 2000  
target_time = 10           
is_running = False
run_start_time = 0

last_update_time = time.ticks_ms()
RPM_UPDATE_MS = 100 # Run PID loop at 10Hz

smoothed_motor_rpm = 0.0

# PID Tuning Parameters (Set to 0 by default; require manual tuning)
Kp = 0.0100 # e.g., 0.0100
Ki = 0.0015 # e.g., 0.0015
Kd = 0.0025 # e.g., 0.0025

integral_error = 0.0
prev_error = 0.0
current_pwm_out = NEUTRAL_PWM
current_error = 0.0

# Touch screen state
old_x, old_y = -1, -1
was_touched = False

# UI Colors (RGB565)
BG_COLOR = ili9341.color565(0, 0, 0)
TEXT_COLOR = ili9341.color565(255, 255, 255)
BTN_COLOR = ili9341.color565(50, 50, 200)       
START_COLOR = ili9341.color565(0, 180, 0)       
STOP_COLOR = ili9341.color565(200, 0, 0)        
LIVE_COLOR = ili9341.color565(0, 255, 255)
CAL_COLOR = ili9341.color565(200, 100, 0)
CROSSHAIR_COLOR = ili9341.color565(255, 0, 255) 


# ==============================================================================
# 4. CONTROL FUNCTIONS
# ==============================================================================
def compute_pid(target_c_rpm, current_c_rpm, dt_sec):
    """Calculates the PID output and combines it with feedforward base PWM."""
    global integral_error, prev_error, current_error
    
    current_error = target_c_rpm - current_c_rpm
    
    # Proportional
    P = Kp * current_error
    
    # Integral (With anti-windup clamping limits)
    integral_error += current_error * dt_sec
    integral_error = max(-5000, min(5000, integral_error)) 
    I = Ki * integral_error
    
    # Derivative
    D = Kd * (current_error - prev_error) / dt_sec
    prev_error = current_error
    
    # Get base feedforward guess
    required_motor_rpm = target_c_rpm * GEAR_RATIO
    base_pwm = estimate_base_pwm(required_motor_rpm)
    
    # Combine Base + Corrections and clamp to safe ESC limits (Forward Only)
    raw_pwm = base_pwm + P + I + D
    return max(1500, min(2000, raw_pwm))

def reset_pid():
    """Clears accumulated errors to prevent unpredictable jumps on start/restart."""
    global integral_error, prev_error, current_error
    integral_error = 0.0
    prev_error = 0.0
    current_error = 0.0

def run_auto_calibration():
    """Blocking routine to dynamically map the hardware's non-linear PWM/RPM curve."""
    global THROTTLE_CURVE, pulse_count, smoothed_motor_rpm
    
    # Setup UI for calibration
    display.clear(BG_COLOR)
    display.draw_text8x8(10, 10, "AUTO CALIBRATING...", LIVE_COLOR)
    display.draw_text8x8(10, 30, "Keep hands clear!", STOP_COLOR)
    
    test_pwms = [KICKSTART_PWM, 1600, 1625, 1650, 1675, 1700, 1800, 1900, 2000]
    new_curve = [(0, 1500)]
    
    # 1. Break static friction
    display.draw_text8x8(10, 60, "Applying Kick-Start...", TEXT_COLOR)
    set_esc_pwm(KICKSTART_PWM)
    time.sleep_ms(KICKSTART_MS)
    
    # 2. Step through test values and record resulting RPM
    for pwm in test_pwms:
        set_esc_pwm(pwm)
        display.fill_rectangle(10, 90, 220, 20, BG_COLOR)
        display.draw_text8x8(10, 90, f"Testing PWM: {pwm}us", TEXT_COLOR)
        
        # Read RPM for 1.5 seconds to let inertia settle
        start_t = time.ticks_ms()
        rpm_sum = 0
        readings = 0
        
        while time.ticks_diff(time.ticks_ms(), start_t) < 1500:
            time.sleep_ms(250) # Wait for PID cycle
            
            # Atomic read of pulse count to avoid interrupt corruption
            state = machine.disable_irq()
            current_pulses = pulse_count
            pulse_count = 0
            machine.enable_irq(state)
            
            # Calculate and smooth RPM
            raw_motor_rpm = ((current_pulses / 0.1) / 2.0) * 60.0
            smoothed_motor_rpm = (raw_motor_rpm * 0.4) + (smoothed_motor_rpm * 0.6)
            
            # Only average the final 1 second of data (drop first 500ms of acceleration)
            if time.ticks_diff(time.ticks_ms(), start_t) > 500:
                rpm_sum += smoothed_motor_rpm
                readings += 1
                
        # Calculate final mapping (1.05 multiplier avoids feedforward overshoot)
        final_rpm = math.ceil(rpm_sum / max(1, readings) * 1.05)
        new_curve.append((final_rpm, pwm))

        print(f"RPM: {final_rpm}, PWM: {pwm}us")
        display.draw_text8x8(10, 110 + (test_pwms.index(pwm)*15), f"-> {int(final_rpm)} M_RPM", LIVE_COLOR)
    
    # 3. Shutdown Motor, Save, and Exit
    set_esc_pwm(NEUTRAL_PWM)
    
    # Ensure list is strictly sorted by RPM for the interpolator logic to work
    new_curve.sort(key=lambda x: x[0])
    THROTTLE_CURVE = new_curve
    
    display.draw_text8x8(10, 250, "CALIBRATION SAVED!", START_COLOR)
    time.sleep(2)
    draw_ui()


# ==============================================================================
# 5. GUI DRAWING FUNCTIONS
# ==============================================================================
def draw_ui():
    """Draws the static elements of the main User Interface."""
    display.clear(BG_COLOR)
    
    display.draw_text8x8(10, 10, "TARGET C_RPM:", TEXT_COLOR)
    display.draw_text8x8(10, 90, "DURATION (sec):", TEXT_COLOR)
    
    # Auto-Calibrate Button 
    display.fill_rectangle(180, 5, 50, 20, CAL_COLOR)
    display.draw_text8x8(190, 11, "CAL", TEXT_COLOR)
    
    # Live Stats Box
    display.fill_rectangle(0, 155, 240, 55, ili9341.color565(30, 30, 30))
    display.draw_text8x8(10, 160, "PWM:", LIVE_COLOR)
    display.draw_text8x8(130, 160, "TIME:", LIVE_COLOR)
    display.draw_text8x8(10, 175, "M_RPM:", LIVE_COLOR)
    display.draw_text8x8(130, 175, "C_RPM:", LIVE_COLOR)
    display.draw_text8x8(10, 190, "RPM_ERR:", LIVE_COLOR)
    
    update_settings_readouts()
    update_live_readouts(NEUTRAL_PWM, target_time if not is_running else 0, 0, 0, 0)
    
    # RPM Adjustment Buttons (100 RPM increments)
    display.fill_rectangle(10, 30, 100, 40, BTN_COLOR)
    display.draw_text8x8(30, 45, "- 100", TEXT_COLOR)
    display.fill_rectangle(130, 30, 100, 40, BTN_COLOR)
    display.draw_text8x8(150, 45, "+ 100", TEXT_COLOR)
    
    # Duration Adjustment Buttons (1 Sec increments)
    display.fill_rectangle(10, 110, 100, 40, BTN_COLOR)
    display.draw_text8x8(40, 125, "- 1", TEXT_COLOR)
    display.fill_rectangle(130, 110, 100, 40, BTN_COLOR)
    display.draw_text8x8(160, 125, "+ 1", TEXT_COLOR)
    
    draw_main_button()

def update_settings_readouts():
    """Updates the static settings text block."""
    display.fill_rectangle(130, 10, 45, 10, BG_COLOR)
    display.draw_text8x8(130, 10, str(target_carrier_rpm), TEXT_COLOR)
    display.fill_rectangle(130, 90, 45, 10, BG_COLOR)
    display.draw_text8x8(130, 90, str(target_time), TEXT_COLOR)

def update_live_readouts(pwm, remaining_sec, m_rpm, c_rpm, err):
    """Updates the dynamic statistics text block."""
    # Blank out old text with dark grey backgrounds
    display.fill_rectangle(45, 160, 75, 10, ili9341.color565(30, 30, 30))
    display.fill_rectangle(175, 160, 60, 10, ili9341.color565(30, 30, 30))
    display.fill_rectangle(65, 175, 60, 10, ili9341.color565(30, 30, 30))
    display.fill_rectangle(185, 175, 50, 10, ili9341.color565(30, 30, 30))
    display.fill_rectangle(80, 190, 80, 10, ili9341.color565(30, 30, 30))
    
    # Write new text
    display.draw_text8x8(45, 160, str(int(pwm)), LIVE_COLOR)
    display.draw_text8x8(175, 160, f"{remaining_sec:.1f}s", LIVE_COLOR)
    display.draw_text8x8(65, 175, str(int(m_rpm)), LIVE_COLOR)
    display.draw_text8x8(185, 175, str(int(c_rpm)), LIVE_COLOR)
    display.draw_text8x8(80, 190, str(int(err)), LIVE_COLOR)

def draw_main_button():
    """Toggles the visual state of the main Start/Stop button."""
    if not is_running:
        display.fill_rectangle(10, 215, 220, 90, START_COLOR)
        display.draw_text8x8(100, 255, "START", TEXT_COLOR)
    else:
        display.fill_rectangle(10, 215, 220, 90, STOP_COLOR)
        display.draw_text8x8(100, 255, "STOP!", TEXT_COLOR)


# ==============================================================================
# 6. STARTUP INSTRUCTIONS
# ==============================================================================
draw_ui()


# ==============================================================================
# 7. MAIN EVENT LOOP
# ==============================================================================
while True:
    current_time = time.ticks_ms()
    
    # --- 10Hz PID & SENSOR CONTROL LOOP ---
    dt_ms = time.ticks_diff(current_time, last_update_time)
    if dt_ms >= RPM_UPDATE_MS:
        dt_sec = dt_ms / 1000.0
        
        # 1. Safely read and clear the pulse count (Atomic block)
        state = machine.disable_irq()
        current_pulses = pulse_count
        pulse_count = 0
        machine.enable_irq(state)
        
        # 2. Calculate Actual RPM
        pulses_per_sec = current_pulses / dt_sec
        raw_motor_rpm = (pulses_per_sec / 2.0) * 60.0 # 2 pulses per revolution
        
        # Exponential smoothing (Low-Pass Filter) to remove sensor jitter
        smoothed_motor_rpm = (raw_motor_rpm * 0.4) + (smoothed_motor_rpm * 0.6)
        carrier_rpm = smoothed_motor_rpm / GEAR_RATIO
        
        # 3. Execution / Timer Logic
        if is_running:
            elapsed_ms = time.ticks_diff(current_time, run_start_time)
            remaining_sec = target_time - (elapsed_ms / 1000.0)
            
            if remaining_sec > 0:
                # OVERRIDE A: KICK-START
                if elapsed_ms < KICKSTART_MS and target_carrier_rpm > 0:
                    current_pwm_out = KICKSTART_PWM
                    reset_pid() # Freeze integral from winding up while we force the motor
                
                # OVERRIDE B: ESC STARTUP DELAY (Anti-Windup)
                elif smoothed_motor_rpm < 60 and target_carrier_rpm > 0:
                    # Motor is taking time to spool up. Use feedforward guess only.
                    required_motor_rpm = target_carrier_rpm * GEAR_RATIO
                    current_pwm_out = estimate_base_pwm(required_motor_rpm)
                    reset_pid() # Freeze integral from accumulating impossible error
                
                # STANDARD RUN: FULL PID CONTROL
                else:
                    current_pwm_out = compute_pid(target_carrier_rpm, carrier_rpm, dt_sec)
                    
                set_esc_pwm(current_pwm_out)
                
            else:
                # Timer complete -> Stop process
                is_running = False
                current_pwm_out = NEUTRAL_PWM
                set_esc_pwm(NEUTRAL_PWM) 
                reset_pid()
                draw_main_button()
        else:
            # Idle state
            current_pwm_out = NEUTRAL_PWM
            current_error = 0
            remaining_sec = target_time
            
        update_live_readouts(current_pwm_out, max(0, remaining_sec), smoothed_motor_rpm, carrier_rpm, current_error)
        last_update_time = current_time
    
    
    # --- TOUCH & UI HANDLING LOOP ---
    if touch_irq.value() == 0:
        was_touched = True
        x_vals, y_vals = [], []
        
        # Sample touch position 6 times to prevent spurious inputs
        for _ in range(6):
            raw = touch.raw_touch()
            if raw is not None:
                x_vals.append(raw[0])
                y_vals.append(raw[1])
                
        if len(x_vals) >= 6:
            # Median filtering
            x_vals.sort()
            y_vals.sort()
            tx, ty = touch.normalize(x_vals[len(x_vals) // 2], y_vals[len(y_vals) // 2])
            
            # Screen orientation mapping
            ty = 320 - ty
            tx = max(10, min(230, tx))
            ty = max(10, min(310, ty))

            # Draw Crosshair
            if abs(tx - old_x) > 1 or abs(ty - old_y) > 1:
                if old_x != -1: 
                    # Erase old crosshair
                    display.draw_hline(old_x - 10, old_y, 21, BG_COLOR)
                    display.draw_vline(old_x, old_y - 10, 21, BG_COLOR)
                    
                # Draw new crosshair
                display.draw_hline(tx - 10, ty, 21, CROSSHAIR_COLOR)
                display.draw_vline(tx, ty - 10, 21, CROSSHAIR_COLOR)
                old_x, old_y = tx, ty

            button_pressed = False
            
            # --- Button Hitbox Logic ---
            if not is_running:
                
                # Auto-Calibrate (Top Right)
                if 170 <= tx <= 240 and 0 <= ty <= 30:
                    run_auto_calibration()
                    was_touched = False
                    continue

                # Target RPM Adjustments 
                if 10 <= tx <= 110 and 30 <= ty <= 70:
                    target_carrier_rpm = max(0, target_carrier_rpm - 100)
                    button_pressed = True
                    
                elif 130 <= tx <= 230 and 30 <= ty <= 70:
                    target_carrier_rpm = min(8000, target_carrier_rpm + 100)
                    button_pressed = True
                    
                # Target Duration Adjustments
                elif 10 <= tx <= 110 and 110 <= ty <= 150:
                    target_time = max(1, target_time - 1)
                    button_pressed = True
                    
                elif 130 <= tx <= 230 and 110 <= ty <= 150:
                    target_time = min(300, target_time + 1)
                    button_pressed = True

            # Start/Stop Button (Bottom)
            if 10 <= tx <= 230 and 215 <= ty <= 305:
                is_running = not is_running
                button_pressed = True
                
                if is_running:
                    run_start_time = time.ticks_ms()
                    reset_pid() 
                else:
                    set_esc_pwm(NEUTRAL_PWM) 
                    current_pwm_out = NEUTRAL_PWM
                    reset_pid()
                
                draw_main_button()

            if button_pressed and not is_running:
                update_settings_readouts()
                
            if button_pressed:
                time.sleep_ms(250) # Debounce UI buttons
                
    elif was_touched:
        # Screen released -> redraw clean UI to clear the crosshair
        draw_ui()
        was_touched = False
        old_x, old_y = -1, -1
            
    time.sleep_ms(5)