import Foundation
import Vision
let request = VNDetectBarcodesRequest()
request.symbologies = [.qr]
request.usesCPUOnly = true
let handler = VNImageRequestHandler(url: URL(fileURLWithPath: CommandLine.arguments[1]), options: [:])
do {
    try handler.perform([request])
    let payloads = (request.results ?? []).compactMap { $0.payloadStringValue }
    FileHandle.standardOutput.write(try JSONSerialization.data(withJSONObject: payloads, options: [.sortedKeys]))
} catch {
    FileHandle.standardError.write(Data("QR decode failed: \(error)\n".utf8))
    exit(1)
}
