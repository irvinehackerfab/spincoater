#include <ESP32Servo.h>
uint8_t pin = 13;
double freq = 50;
double stop_duty = 0.05;
ESP32PWM pwm;

void setup() {
	ESP32PWM::allocateTimer(0);
	Serial.begin(115200);
	pwm.attachPin(pin, freq, 16);
  pwm.writeScaled(stop_duty);
}

void loop() {
}
