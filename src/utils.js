export async function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Gets the SDP offer from the RTC peer connection.
 * @param {RTCPeerConnection} rtcConnection - The RTC peer connection instance.
 * @returns {Promise<string>} A promise that resolves with the SDP offer.
 */
export async function get_offer(rtcConnection) {
  const sessionDescription = await rtcConnection.createOffer();
  await rtcConnection.setLocalDescription(sessionDescription);
  await waitToCompleteIceGathering(rtcConnection);
  return rtcConnection.localDescription.sdp;
}

/**
 * Gets the SDP answer from the RTC peer connection.
 * @param {RTCPeerConnection} rtcConnection - The RTC peer connection instance.
 * @returns {Promise<string>} A promise that resolves with the SDP answer.
 */
export async function get_answer(rtcConnection) {
  const sessionDescription = await rtcConnection.createAnswer();
  await rtcConnection.setLocalDescription(sessionDescription);
  await waitToCompleteIceGathering(rtcConnection);
  return rtcConnection.localDescription.sdp;
}

/**
 * Waits for ICE gathering to complete on the RTC peer connection.
 * @param {RTCPeerConnection} rtcConnection - The RTC peer connection instance.
 * @returns {Promise<RTCSessionDescription | undefined>} A promise that resolves with the local description
 * when ICE gathering is complete.
 */
function waitToCompleteIceGathering(rtcConnection) {
  rtcConnection.restartIce();

  return new Promise((resolve) => {
    rtcConnection.addEventListener("icegatheringstatechange", (event) => {
      if (
        event &&
        event.target &&
        "iceGatheringState" in event.target &&
        event.target.iceGatheringState === "complete"
      ) {
        resolve(rtcConnection.localDescription);
      }
    });
  });
}
