constexpr uint8_t PIN { 26 };
void setup() {
  // put your setup code here, to run once:
  ledcAttach(PIN, 490, 20);
  Serial.begin(9600);
  Serial.println("Starting PWM.");
}

void loop() {
  // put your main code here, to run repeatedly:
  // Max pulse width is 2^20-1
  // Period in microseconds is 1/490 * 10^6
  // Brake pulse width is 1060 microseconds
  // Max power pulse width is 1860 microseconds
  // This code sets the power from 0% to 100% repeatedly.
  ledcWrite(PIN, 544630);
  Serial.println("PWM: 0%");
  sleep(1);
  ledcWrite(PIN, 647390);
  Serial.println("PWM: 25%");
  sleep(1);
  ledcWrite(PIN, 750151);
  Serial.println("PWM: 50%");
  sleep(1);
  ledcWrite(PIN, 852911);
  Serial.println("PWM: 75%");
  sleep(1);
  ledcWrite(PIN, 955671);
  Serial.println("PWM: 100%");
  sleep(1);
}
