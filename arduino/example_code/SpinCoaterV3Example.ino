#include <ServoTimer2.h>

// Pins
const int ESCPin = 5;
const int hallPin = 2;

// Constants
const int motorPolePairs = 2;
const float gearRatio = 20.f / 74.f;

// Globals
ServoTimer2 ESC;
unsigned long lastHallTime = 0;
unsigned long lastStallTime = 0;
unsigned long lastPrintTime = 0;
bool lastHallState = LOW;
int hallStateChanges = 0;
float currentPWM = 1500.f;

bool enableMotor = false;

// Class definitions
class PID {
public:
  PID(float newKP, float newKI, float newKD) {
    KP = newKP;
    KI = newKI;
    KD = newKD;
  }

  void setKP(float newKP) {
    KP = newKP;
  }

  void setKI(float newKI) {
    KI = newKI;
  }

  void setKD(float newKD) {
    KD = newKD;
  }

  void reset() {
    currentIntegral = 0.f;
    currentDerivative = 0.f;
  }

  // Gives error and outputs PID, deltaTime in seconds
  float Update(float currentError, float deltaTime) {
    currentIntegral += currentError * deltaTime;
    currentDerivative = (currentError - previousError) / deltaTime;

    previousError = currentError;
    return KP * currentError + KI * currentIntegral + KD * currentDerivative;
  }

public:
  float KP = 0.f;
  float KI = 0.f;
  float KD = 0.f;

  float currentIntegral = 0.f;
  float currentDerivative = 0.f;
  float previousError = 0.0;
};

// Global class variables
const float P = 0.000015f;//0.000050f;
const float I = 0.000000f;
const float D = 0.000000f;
PID motorPID(P, I, D);

// Functions
void setup() {
  Serial.begin(9600);

  pinMode(hallPin, INPUT_PULLUP);

  ESC.attach(ESCPin);
  ESC.write(1500); // Send "stop" signal to ESC.

  delay(1000); // Delay to allow the ESC to recognize the stopped signal
  Serial.println("Ready. Send 0 to stop motor or 1 to start motor and any other number to set RPM. 500-10,000 RPM range at 12V");
}

float motorFrequency = 0.f;
float motorRPM = 0.f;
float plateRPM = 0.f;

float RPMGoal = 600.f;

void hallStateChanged(bool currentHallState) {
  // Update state
  hallStateChanges++;

  if (currentHallState == HIGH) {
    // Time
    unsigned long currentTime = micros();
    unsigned long microDeltaTime = currentTime - lastHallTime;

    // Compute frequency
    motorFrequency = (1000000.f / motorPolePairs) / microDeltaTime;
    motorRPM = motorFrequency * 60.f;
    plateRPM = motorRPM * gearRatio;

    // PID loop
    const float currentError = RPMGoal - plateRPM;
    const float percentError = (currentError * 100.f) / RPMGoal;
    /*
    if (abs(percentError) < 10.f) {
      motorPID.setKP(0.f);
      motorPID.setKI(0.0000025f);
      motorPID.setKD(0.0000025f);
    } else {
      motorPID.setKP(P);
      motorPID.setKI(I);
      motorPID.setKD(D);
      motorPID.reset();
    }
    */

    float PWMAcceleration = motorPID.Update(currentError, microDeltaTime / 1000000.f);
    PWMAcceleration = constrain(PWMAcceleration, -0.0250f, 0.500f);
    currentPWM = constrain(currentPWM + PWMAcceleration, 1500, 2100);

    if (currentTime - lastPrintTime > 500000) {
      Serial.println("Error:" + String(percentError) + "%, RPM:" + String(plateRPM) + ", Accel:" + String(PWMAcceleration * 100) + ", PWM:" + String(currentPWM));
      //Serial.println("motorFrequency:" + String(motorFrequency) + ", RPM:" + String(plateRPM) + ", motorFrequency*GR:" + String(motorFrequency * gearRatio));

      lastPrintTime = currentTime;
    }

    ESC.write(floor(currentPWM));

    // End
    lastHallTime = currentTime;
  }
}

void loop() {
  if (Serial.available() > 0) {
    int incomingByte = Serial.parseInt(); 

    switch (incomingByte) {
      case 0:
      enableMotor = false;
      Serial.println("Disabled motor!");
      break;
      
      case 1:
      enableMotor = true;
      currentPWM = 1580;
      Serial.println("Enabled motor!");
      break;

      case 2:
      enableMotor = true;
      currentPWM = 1580;
      Serial.println("Enabled motor!");
      break;

      default:
      RPMGoal = incomingByte;
      Serial.println("Set RPMGoal to: " + String(RPMGoal));
      break;
    }

    /*
    if (incomingByte  < (1500 - 600) || incomingByte  > (1500 + 600)) {
      Serial.println("Not valid!");
    } else {
      Serial.println(incomingByte);
      //ESC.writeMicroseconds(incomingByte); // Send signal to ESC.
      ESC.write(incomingByte);
    }
    */

    //Serial.println("Enter PWM signal value 1000 to 2000, 1500 to stop");
  }
  
  if (enableMotor) {
    unsigned long currentTime = millis();
    
    unsigned long deltaStallTime = currentTime - lastStallTime;
    if (deltaStallTime > 2000) {
      currentPWM = 1580; // Kick-start the motor
      ESC.write(currentPWM);
      Serial.println("Kick starting motor...");
    }

    const bool currentHallState = digitalRead(hallPin);
    if (currentHallState != lastHallState) {
      hallStateChanged(currentHallState);
      lastStallTime = currentTime;
    }
    lastHallState = currentHallState;
  } else {
    ESC.write(1500);
  }
}