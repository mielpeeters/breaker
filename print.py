import matplotlib.pyplot as plt


# Process data and create counts
pipeline_count = 0
audio_engine_count = 0
x_pipeline = []
y_pipeline = []
x_audio_engine = []
y_audio_engine = []

with open('data_out2.csv', 'r') as f:
    data = f.read()

for line in data.strip().split('\n'):
    item, timestamp = line.split(', ')
    timestamp = int(timestamp)

    if item == 'pipeline':
        pipeline_count += 1
        x_pipeline.append(timestamp)
        y_pipeline.append(pipeline_count)
    elif item == 'audio_engine':
        audio_engine_count += 1
        x_audio_engine.append(timestamp)
        y_audio_engine.append(audio_engine_count)

# Plot the data
plt.plot(x_pipeline, y_pipeline, label='pipeline')
plt.plot(x_audio_engine, y_audio_engine, label='audio_engine')

# Add labels and legend
plt.xlabel('Time')
plt.ylabel('Count')
plt.legend()

# Show the plot
plt.show()
