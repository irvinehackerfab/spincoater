// madhephaestus/ESP32Servo: Arduino-compatible servo library for the ESP32
// TaskScheduler | Arduino Documentation

#include <ESP32Servo.h>
#include "GUIv3_GSLC.h"

// === ESP32 pin mapping  ===
// See the user guide for all pin descriptions:
// https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html

// 0, RX, TX, EN, 12, 13, 14, 15 and 3V3 may be used for the ESP-PROG-2.

// Display pins
// These are set directly in the library, not here.
// See this issue for more information:
// https://github.com/irvinehackerfab/spincoater/issues/6
// TFT_MISO and T_DO: 19
// TFT_MOSI and T_DIN: 23
// TFT_SCK and T_CLK: 18
// TFT_CS: 16
// TFT_DC: 2
// TFT_RST: 4
// TOUCH_CS: 33
#define LED 22
// The LCD is powered with the 5V pin.

// TODO: Remove all usages of the push buttons now that we have a touchscreen
// Buttons (use INPUT_PULLUP)
#define PIN_RPM_UP    25
#define PIN_RPM_DOWN  26
#define PIN_TIME_UP   27
#define PIN_TIME_DOWN 34
#define PIN_START     35

// Motor (servo) pin
#define PIN_MOTOR 32

// Hall effect sensor (interrupt)
#define PIN_HE 17

// Motor Spinning Constants
constexpr int preSpinRPM = 600;
constexpr int rampTime = 1000;
constexpr int rampSteps = 50;
constexpr int maxRPM = 12000;

// HE Sensor Variables
volatile unsigned long motorRevolutionsDoubled;
unsigned long previousTimeMillis;
constexpr double adj_mtr = 1.42857143;

Servo servo; // Setup the Servo

// Saves Data for the Spin Phase
struct spinState {
  long long rpm = 3000;
  long long duration = 30;
};

bool debounce(int);
void setSpin(int, int);
spinState menuLoop();
void preSpin();
void Spin(int rpm, int duration);
void halfRevolutionInterrupt();
int readRpm();
int mapRPM(int);
void updateValues();

// Handles the interrupts of the HE sensor.
// Every 2 counts is one motor revolution.
void IRAM_ATTR halfRevolutionInterrupt()
{
  // Incrementing is ok here because interrupts are disabled in readRpm().
  motorRevolutionsDoubled++;
}

// Uses the HE Sensor Data to calculate RPM
// This function is only reliable if the program has been running for less than 50 days.
// For more info, see https://docs.arduino.cc/language-reference/en/functions/time/millis/
int readRpm(){
  // For why we need to disable interrupts, see https://www.gammon.com.au/interrupts
  noInterrupts();
  unsigned long motorRevolutionsDoubledClone = motorRevolutionsDoubled;
  motorRevolutionsDoubled = 0;
  interrupts();
  unsigned long elapsedTimeMillis = millis() - previousTimeMillis;
  previousTimeMillis = millis();
  unsigned long measuredRpm;
  if (elapsedTimeMillis > 0) {
    // (2*motor revolutions) * 1/2 * (20 plate revolutions / 74 motor revolutions) * 1/(elapsedTimeMillis ms) * (6000 ms / 1 min)
    // = (2*motor revolutions) * 30,000 / (37 * elapsedTimeMillis)
    Serial.print("Motor revs: ");
    Serial.println(motorRevolutionsDoubled);
    Serial.print("Millis: ");
    Serial.println(elapsedTimeMillis);
    // Final units: plate revolutions per minute
    measuredRpm = motorRevolutionsDoubledClone * 30000 / (37 * elapsedTimeMillis);
  } else {
    measuredRpm = 0;
  }
  return measuredRpm;
}

// Debounce any of the button presses
bool prev_button_states[5] = { false, false, false, false, false };
bool button_states[5] = { false, false, false, false, false };
bool debounce(int buttonNumber) {
  bool state;
  switch (buttonNumber) {
    case 0: state = digitalRead(PIN_RPM_UP); break;
    case 1: state = digitalRead(PIN_RPM_DOWN); break;
    case 2: state = digitalRead(PIN_TIME_UP); break;
    case 3: state = digitalRead(PIN_TIME_DOWN); break;
    case 4: state = digitalRead(PIN_START); break;
    default: return false;
  }
  button_states[buttonNumber] = !state;

  if (button_states[buttonNumber] != prev_button_states[buttonNumber]) {
    prev_button_states[buttonNumber] = button_states[buttonNumber];
    if (button_states[buttonNumber]) {
      delay(50);
      return true;
    }
  }
  return false;
}

// Maps input RPM to a value that is understandable by the Servo
int mapRPM(int x){
  // map returns long; ceil used in original — keep same behavior
  return ceil(map(x, 0, maxRPM, 1500, 2000));
}

// Transitions between the current RPM (curr) and the target RPM (target)
void setSpin(int curr, int target){
  float deltaR = (float) (target - curr) / rampSteps;
  float deltaT = (float) (rampTime) / rampSteps;
  for(int i = 0; i < rampSteps; i++){
    int rpm = (int)(curr + deltaR * i);
    servo.writeMicroseconds(mapRPM(rpm));
    delay((int)deltaT);
  }
  servo.writeMicroseconds(mapRPM(target));
}

// Menu Loop (User Selects RPM and Duration)
spinState menuLoop(){
  spinState state;
  bool initial = true;
  while(true){
    bool updated = true;
    if(debounce(1)){ // Increases RPM
      state.rpm += 100;
      Serial.println("RPM Up");
    }
    else if(debounce(0)){ // Decreases RPM
      state.rpm -= 100;
      Serial.println("RPM Down");
    }
    else if (debounce(2)) { // Increases Duration by 1 Second
        Serial.println("Duration Up");
        state.duration += 1;
    }
    else if (debounce(3)) { // Decreases Duration by 1 Second
        Serial.println("Duration Down");
        state.duration -= 1;
    }
    else{
      updated = false;
    }

    if(debounce(4)){ // Start Button: Returns the RPM and Duration data for later use
      return state;
    }

    // Updates the Display with the Current Values
    if(updated || initial){
      initial = false;
    }
  }
}

// Controls the PreSpin Phase (Add Photoresist here)
void preSpin(){
  setSpin(0, preSpinRPM);
  while(1){
    if (debounce(4)){ // Exit using Start Button
      return;
    }
  }
}

// Controls Spin Phase & Display
void Spin(int rpm, int duration){
  setSpin(preSpinRPM, rpm);
  // int progress = 0;
  unsigned int startTimeMillis = millis();
  // int lastDisplayed = -1;

  // while(progress < duration * 1000){
  //   progress = millis() - startTime;
  //   int timeLeft = ceil(duration - progress/1000.0);
  //   if(timeLeft != lastDisplayed){
  //     lastDisplayed = timeLeft;
  //   }
  //   if(debounce(4)){break;} // Early Exit with Start Button
  //   rpm = readRpm();
  //   if (rpm != 0) {
  //     // Serial.println(rpm);
  //   }
  // }
  while (!debounce(4)) {
      // Waiting for 90 ms means you multiply the "motorRevolutionsDoubled" by 9.009,
      // which hopefully doesn't lose much precision.
      delay(90);
      rpm = readRpm();
      Serial.println(rpm);
  }
  setSpin(rpm, 0);
  delay(3000);
}

void setup() {
  // Serial and pins
  Serial.begin(115200);

  pinMode(PIN_RPM_UP, INPUT_PULLUP);
  pinMode(PIN_RPM_DOWN, INPUT_PULLUP);
  pinMode(PIN_TIME_UP, INPUT_PULLUP);
  pinMode(PIN_TIME_DOWN, INPUT_PULLUP);
  pinMode(PIN_START, INPUT_PULLUP);

  // Hall effect sensor pin
  pinMode(PIN_HE, INPUT_PULLUP);

  // Servo attach: the ESP32 Servo
  pinMode(PIN_MOTOR, OUTPUT);
  digitalWrite(PIN_MOTOR, LOW);       // hold LOW during boot
  delay(200);                          // short delay for stable boot
  servo.attach(PIN_MOTOR, 1000, 2000); // then attach servo
  servo.writeMicroseconds(1500); // for esc

  // Setup display
  gslc_InitDebug(&DebugOut);
  InitGUIslice_gen();
  pinMode(LED, OUTPUT);
  digitalWrite(LED, HIGH);

  // Setup HE Sensor interrupt using the pin number
  motorRevolutionsDoubled = 0;
  previousTimeMillis = millis();
  attachInterrupt(digitalPinToInterrupt(PIN_HE), halfRevolutionInterrupt, RISING);

  Serial.println("ESP32 Spin Coater ready");
}

int n = 30;
int data[30];
void test(){
  int step = 100;
  int initial = 0;
  setSpin(0, initial);

  // data[j] is each 30 data points and initial + step * (i+1) is the RPM
  for(int i = 0; i < 50; i++){
    int ret = 0;
    setSpin(initial+step * i , initial + step * (i+1) );
    for(int j = 0; j < 30; j++ ){
      delay(1000);
      data[j] = readRpm();
      // add data[j]
    }
    Serial.println(initial + step * (i+1));
  }
  setSpin(5000, 0);
}

// System Loop
void loop() {
  // test(); // These two lines are for testing/graphing
  //while(1){}
  while(1){
    gslc_Update(&m_gui);
    spinState state = menuLoop();
    Serial.println("Finished Menu State");
    preSpin();
    Spin((int)state.rpm, (int)state.duration);
  }
}
