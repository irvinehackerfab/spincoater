// See this link for the LEDC documentation:
// https://docs.espressif.com/projects/arduino-esp32/en/latest/api/ledc.html

uint8_t PWM_PIN = 23;
uint32_t FREQUENCY = 50;
// 5% of 255 = 12.75 = 12
uint32_t STOP_DUTY = 12;

void setup() {
  // Initialize serial communication at 115200 bits per second:
  Serial.begin(115200);

  // Setup timer with given frequency, resolution and attach it to a led pin with auto-selected channel
  ledcAttach(PWM_PIN, FREQUENCY, LEDC_TIMER_16_BIT);
  // Set duty cycle
  ledcWrite(PWM_PIN, STOP_DUTY);
  Serial.print("Current duty: ");
  Serial.println(ledcRead(PWM_PIN));
}

void loop() {
  // put your main code here, to run repeatedly:

}
